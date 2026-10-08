//! One task per added board: connect, `hello` with the token, then a `ping` every second for as
//! long as the board answers. The board switches everything off after 3 s without a message, so
//! these pings are also what keeps its outputs allowed to run.
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep, timeout, MissedTickBehavior};
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::protocol::{self, Incoming, Outgoing};
use super::{LinkCommand, LinkStatus};

pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const WELCOME_TIMEOUT: Duration = Duration::from_secs(5);
const PING_INTERVAL: Duration = Duration::from_secs(1);
const MAX_MISSED_PONGS: u32 = 3;
const MIN_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(30);

enum End {
    /// Connection lost or never made. `true` when the session had got as far as `welcome`.
    Lost(bool),
    /// The board refused our token.
    Rejected,
    BadProto,
    /// Unpaired on our request, or told to stop.
    Done
}

pub fn ws_url(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("ws://[{host}]:{port}{}", protocol::PATH)
    } else {
        format!("ws://{host}:{port}{}", protocol::PATH)
    }
}

pub async fn connect(host: &str, port: u16) -> Result<Socket, String> {
    match timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(ws_url(host, port))).await {
        Ok(Ok((socket, _))) => Ok(socket),
        Ok(Err(err)) => Err(err.to_string()),
        Err(_) => Err("timed out".into())
    }
}

pub async fn send(socket: &mut Socket, message: &Outgoing<'_>) -> Result<(), String> {
    socket.send(Message::text(protocol::encode(message))).await.map_err(|e| e.to_string())
}

/// The close code of a close frame, if it has one.
pub fn close_code(frame: &Option<CloseFrame>) -> Option<u16> {
    frame.as_ref().map(|f| u16::from(f.code))
}

pub fn spawn(app: AppHandle, id: String) -> mpsc::UnboundedSender<LinkCommand> {
    let (tx, rx) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(run(app, id, rx));
    tx
}

async fn run(app: AppHandle, id: String, mut commands: mpsc::UnboundedReceiver<LinkCommand>) {
    let mut backoff = MIN_BACKOFF;

    loop {
        let Some(record) = super::record(&app, &id) else {
            return;
        };

        match session(&app, &record.id, &record.host, record.port, &record.token, &mut commands).await {
            End::Done => return,
            End::Rejected => {
                tracing::warn!("ESP32 board {id} refused its token, it needs pairing again");
                super::set_status(&app, &id, LinkStatus::NeedsPairing);
                return wait_for_shutdown(&mut commands).await;
            }
            End::BadProto => {
                tracing::warn!("ESP32 board {id} uses another protocol version");
                super::set_status(&app, &id, LinkStatus::UpdateFirmware);
                return wait_for_shutdown(&mut commands).await;
            }
            End::Lost(was_online) => {
                super::set_status(&app, &id, LinkStatus::Offline);
                if was_online {
                    backoff = MIN_BACKOFF;
                }
            }
        }

        // Retry after the backoff, or right away when mDNS announces the board again.
        tokio::select! {
            _ = sleep(backoff) => {}
            command = commands.recv() => match command {
                Some(LinkCommand::Address) => backoff = MIN_BACKOFF,
                // Offline: there's no board to tell. The user can unpair it on the board itself.
                Some(LinkCommand::Unpair) | Some(LinkCommand::Shutdown) | None => return,
                // The config goes out after the next welcome anyway, and a test or a stop for a
                // board that isn't there has nothing to do.
                Some(LinkCommand::Config)
                | Some(LinkCommand::Test(_))
                | Some(LinkCommand::Stop)
                | Some(LinkCommand::Output(_)) => {}
            }
        }
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

async fn wait_for_shutdown(commands: &mut mpsc::UnboundedReceiver<LinkCommand>) {
    while let Some(command) = commands.recv().await {
        if matches!(command, LinkCommand::Unpair | LinkCommand::Shutdown) {
            return;
        }
    }
}

fn end_for_close(code: Option<u16>, online: bool) -> End {
    match code {
        Some(protocol::CLOSE_UNAUTHORIZED) | Some(protocol::CLOSE_UNPAIRED_ON_BOARD) => End::Rejected,
        Some(protocol::CLOSE_BAD_PROTO) => End::BadProto,
        _ => End::Lost(online)
    }
}

async fn session(
    app: &AppHandle,
    id: &str,
    host: &str,
    port: u16,
    token: &str,
    commands: &mut mpsc::UnboundedReceiver<LinkCommand>
) -> End {
    let mut socket = match connect(host, port).await {
        Ok(socket) => socket,
        Err(err) => {
            tracing::debug!("ESP32 board {id} at {host}:{port} unreachable: {err}");
            return End::Lost(false);
        }
    };

    if send(&mut socket, &Outgoing::Hello { proto: protocol::PROTO, token }).await.is_err() {
        return End::Lost(false);
    }

    // Wait for `welcome`.
    let welcome = timeout(WELCOME_TIMEOUT, async {
        while let Some(message) = socket.next().await {
            match message {
                Ok(Message::Text(text)) => match protocol::decode(&text) {
                    Some(Incoming::Welcome(welcome)) => return Ok(welcome),
                    Some(Incoming::Err { code, .. }) if code == "bad-proto" => return Err(End::BadProto),
                    _ => {}
                },
                Ok(Message::Close(frame)) => return Err(end_for_close(close_code(&frame), false)),
                Ok(_) => {}
                Err(_) => return Err(End::Lost(false))
            }
        }
        Err(End::Lost(false))
    })
    .await;

    let welcome = match welcome {
        Ok(Ok(welcome)) => welcome,
        Ok(Err(end)) => return end,
        Err(_) => return End::Lost(false)
    };

    if welcome.id != id {
        tracing::warn!("Expected ESP32 board {id} at {host}:{port} but found {}", welcome.id);
        return End::Lost(false);
    }

    super::apply_welcome(app, id, &welcome);

    // The app is the source of truth for the outputs once they've been saved here.
    if let Some(config) = super::config_message(app, id) {
        if send(&mut socket, &config).await.is_err() {
            return End::Lost(false);
        }
    }

    super::set_status(app, id, LinkStatus::Online);
    tracing::info!("ESP32 board {} ({id}) online at {host}:{port}", welcome.name);

    let mut ticker = interval(PING_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut missed = 0u32;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if missed >= MAX_MISSED_PONGS {
                    tracing::info!("ESP32 board {id} stopped answering");
                    return End::Lost(true);
                }
                if send(&mut socket, &Outgoing::Ping).await.is_err() {
                    return End::Lost(true);
                }
                missed += 1;
            }
            message = socket.next() => match message {
                Some(Ok(Message::Text(text))) => match protocol::decode(&text) {
                    Some(Incoming::Pong) => missed = 0,
                    Some(Incoming::Err { code, reference, pin }) => {
                        tracing::warn!("ESP32 board {id} error: {code} (ref {reference:?}, pin {pin:?})");
                        super::emit_error(app, id, code, pin);
                    }
                    _ => {}
                },
                Some(Ok(Message::Close(frame))) => return end_for_close(close_code(&frame), true),
                Some(Ok(_)) => {}
                Some(Err(_)) | None => return End::Lost(true)
            },
            command = commands.recv() => match command {
                // Still connected: nothing to do.
                Some(LinkCommand::Address) => {}
                Some(LinkCommand::Config) => {
                    if let Some(config) = super::config_message(app, id) {
                        if send(&mut socket, &config).await.is_err() {
                            return End::Lost(true);
                        }
                        tracing::info!("Sent the output config to ESP32 board {id}");
                    }
                }
                Some(LinkCommand::Test(pin)) => {
                    let set = Outgoing::Set { pin, on: true, ms: super::TEST_PULSE_MS };
                    if send(&mut socket, &set).await.is_err() {
                        return End::Lost(true);
                    }
                }
                Some(LinkCommand::Stop) => {
                    if send(&mut socket, &Outgoing::Stop).await.is_err() {
                        return End::Lost(true);
                    }
                }
                Some(LinkCommand::Output(message)) => {
                    let text = serde_json::to_string(&message).unwrap_or_default();
                    if socket.send(Message::text(text)).await.is_err() {
                        return End::Lost(true);
                    }
                }
                Some(LinkCommand::Unpair) => {
                    let _ = send(&mut socket, &Outgoing::Unpair).await;
                    // The board answers by closing with 1000. Wait briefly for it.
                    let _ = timeout(Duration::from_secs(2), async {
                        while let Some(Ok(message)) = socket.next().await {
                            if matches!(message, Message::Close(_)) {
                                break;
                            }
                        }
                    })
                    .await;
                    tracing::info!("ESP32 board {id} unpaired");
                    return End::Done;
                }
                Some(LinkCommand::Shutdown) | None => {
                    let _ = socket.close(None).await;
                    return End::Done;
                }
            }
        }
    }
}
