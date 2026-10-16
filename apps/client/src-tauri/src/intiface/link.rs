//! One task per Intiface URL while it's switched on: connect, handshake, list devices, start
//! scanning, read batteries every 30 s, and try again 3 s after any drop or failure. A failure
//! before the handshake is diagnosed (see `super::diagnose`), which is also where auto-launch
//! happens.
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio::time::{interval_at, sleep, Instant, MissedTickBehavior};
use tokio_tungstenite::tungstenite::Message;

use super::protocol::{self, Incoming};
use super::{LinkCommand, Status};
use crate::net::Socket;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const RECONNECT_DELAY: Duration = Duration::from_secs(3);
const BATTERY_POLL_INTERVAL: Duration = Duration::from_secs(30);
/// How long a launch gets to start Intiface's server before auto-launch tries again.
pub const LAUNCH_RETRY_DELAY: Duration = Duration::from_secs(15);

enum End {
    /// Dropped after a successful handshake.
    Dropped,
    /// Failed before the handshake, with Intiface's error if it sent one.
    Failed(String),
    Done
}

pub fn spawn(app: AppHandle, id: u64, url: String) -> mpsc::UnboundedSender<LinkCommand> {
    let (tx, rx) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(run(app, id, url, rx));
    tx
}

async fn run(app: AppHandle, id: u64, url: String, mut commands: mpsc::UnboundedReceiver<LinkCommand>) {
    loop {
        let end = session(&app, id, &url, &mut commands).await;
        super::set_devices(&app, id, |devices| devices.clear());

        match end {
            End::Done => return,
            End::Dropped => {
                tracing::info!("Lost the connection to Intiface");
                super::set_status(&app, id, Status::Connecting);
            }
            End::Failed(reason) => super::diagnose(&app, id, &url, reason).await
        }

        let retry = sleep(RECONNECT_DELAY);
        tokio::pin!(retry);
        loop {
            tokio::select! {
                _ = &mut retry => break,
                command = commands.recv() => match command {
                    Some(LinkCommand::Shutdown) | None => return,
                    Some(LinkCommand::Send(_)) => {}
                }
            }
        }
    }
}

struct Session {
    socket: Socket,
    next_id: u32
}

impl Session {
    async fn send(&mut self, messages: Vec<Value>) -> Result<(), String> {
        let messages: Vec<Value> = messages
            .into_iter()
            .map(|message| {
                let id = self.next_id;
                self.next_id += 1;
                protocol::with_id(message, id)
            })
            .collect();
        self.socket.send(Message::text(Value::Array(messages).to_string())).await.map_err(|e| e.to_string())
    }
}

async fn session(app: &AppHandle, id: u64, url: &str, commands: &mut mpsc::UnboundedReceiver<LinkCommand>) -> End {
    // Keep showing launching, refused or busy across retries instead of flickering every 3 s.
    let shown = super::intiface(app).inner.lock().unwrap().status;
    if !matches!(shown, Some(Status::Launching | Status::Refused | Status::Busy)) {
        super::set_status(app, id, Status::Connecting);
    }

    let socket = match crate::net::connect(url, CONNECT_TIMEOUT).await {
        Ok(socket) => socket,
        Err(err) => {
            tracing::debug!("Intiface at {url} unreachable: {err}");
            return End::Failed(String::new());
        }
    };
    let mut session = Session { socket, next_id: 1 };

    if session.send(vec![protocol::request_server_info()]).await.is_err() {
        return End::Failed(String::new());
    }

    let mut handshake_done = false;
    let mut battery = interval_at(Instant::now() + BATTERY_POLL_INTERVAL, BATTERY_POLL_INTERVAL);
    battery.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let lost = |handshake_done: bool, reason: String| if handshake_done { End::Dropped } else { End::Failed(reason) };

    loop {
        tokio::select! {
            _ = battery.tick(), if handshake_done => {
                if session.send(battery_reads(app)).await.is_err() {
                    return End::Dropped;
                }
            }
            message = session.socket.next() => {
                let text = match message {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return lost(handshake_done, String::new()),
                    Some(Ok(_)) => continue
                };
                let mut reads = false;
                for incoming in protocol::decode(text.as_str()) {
                    match incoming {
                        Incoming::ServerInfo => {
                            handshake_done = true;
                            tracing::info!("Connected to Intiface at {url}");
                            super::set_status(app, id, Status::Connected);
                            // Bluetooth toys only show up once the server scans for them.
                            let start = vec![
                                protocol::message("RequestDeviceList", json!({})),
                                protocol::message("StartScanning", json!({}))
                            ];
                            if session.send(start).await.is_err() {
                                return End::Dropped;
                            }
                        }
                        Incoming::DeviceList(infos) => {
                            super::set_devices(app, id, |devices| {
                                devices.clear();
                                for device in infos.iter().map(protocol::DeviceInfo::to_device) {
                                    if !device.actuators.is_empty() {
                                        devices.insert(device.index, device);
                                    }
                                }
                            });
                            reads = true;
                        }
                        Incoming::DeviceAdded(info) => {
                            let device = info.to_device();
                            if !device.actuators.is_empty() {
                                tracing::info!("Toy connected: {}", device.name);
                                super::set_devices(app, id, |devices| {
                                    devices.insert(device.index, device);
                                });
                                reads = true;
                            }
                        }
                        Incoming::DeviceRemoved(index) => super::set_devices(app, id, |devices| {
                            devices.remove(&index);
                        }),
                        Incoming::Battery { device, level } => super::set_devices(app, id, |devices| {
                            if let Some(device) = devices.get_mut(&device) {
                                device.battery = Some(level);
                            }
                        }),
                        Incoming::Error(reason) => {
                            tracing::warn!("Intiface error: {reason}");
                            // Rejected during the handshake: close and find out why.
                            if !handshake_done {
                                let _ = session.socket.close(None).await;
                                return End::Failed(reason);
                            }
                        }
                        Incoming::Other => {}
                    }
                }
                // A battery level isn't pushed by the server, so ask whenever the list changes.
                if reads && session.send(battery_reads(app)).await.is_err() {
                    return End::Dropped;
                }
            }
            command = commands.recv() => match command {
                Some(LinkCommand::Send(messages)) => {
                    if handshake_done && session.send(messages).await.is_err() {
                        return End::Dropped;
                    }
                }
                Some(LinkCommand::Shutdown) | None => {
                    let _ = session.socket.close(None).await;
                    return End::Done;
                }
            }
        }
    }
}

fn battery_reads(app: &AppHandle) -> Vec<Value> {
    super::devices(app).iter().filter_map(protocol::battery_read).collect()
}
