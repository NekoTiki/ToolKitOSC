//! One task per connection to bHaptics Player while the add-on is on: connect, pass Player's
//! answers to `super`, send what the tick hands over, and try again 3 s after any drop or failure.
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;

use super::protocol;
use super::{LinkCommand, Status};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const RECONNECT_DELAY: Duration = Duration::from_secs(3);

enum End {
    /// Dropped after connecting.
    Dropped,
    /// Nothing answered.
    Failed,
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
        match end {
            End::Done => return,
            End::Dropped => {
                tracing::info!("Lost the connection to bHaptics Player");
                super::set_status(&app, id, Status::Connecting);
            }
            End::Failed => super::set_status(&app, id, Status::NotRunning)
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

async fn session(app: &AppHandle, id: u64, url: &str, commands: &mut mpsc::UnboundedReceiver<LinkCommand>) -> End {
    let mut socket = match crate::net::connect(url, CONNECT_TIMEOUT).await {
        Ok(socket) => socket,
        Err(err) => {
            tracing::debug!("bHaptics Player at {url} unreachable: {err}");
            return End::Failed;
        }
    };
    tracing::info!("Connected to bHaptics Player");
    super::set_status(app, id, Status::Connected);
    // Player answers any packet with what's connected.
    if socket.send(Message::text(protocol::packet(&[]))).await.is_err() {
        return End::Dropped;
    }

    loop {
        tokio::select! {
            message = socket.next() => match message {
                Some(Ok(Message::Text(text))) => {
                    if let Some(response) = protocol::decode(text.as_str()) {
                        super::set_player(app, id, response);
                    }
                }
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return End::Dropped,
                Some(Ok(_)) => {}
            },
            command = commands.recv() => match command {
                Some(LinkCommand::Send(text)) => {
                    if socket.send(Message::text(text)).await.is_err() {
                        return End::Dropped;
                    }
                }
                Some(LinkCommand::Shutdown) | None => {
                    let _ = socket.close(None).await;
                    return End::Done;
                }
            }
        }
    }
}
