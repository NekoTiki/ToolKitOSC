//! One task per server URL: connect, `auth-token` first when signed in, then a text `ping` every
//! 5 s. No answer within 5 s, or a dropped socket, means reconnecting 3 s later, forever, since this
//! is the host's only way to reach its viewers. `auth-error` and `protocol-mismatch` stop it: the
//! same token or app version would only be refused again.
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio::time::{interval_at, sleep, Instant, MissedTickBehavior};
use tokio_tungstenite::tungstenite::Message;

use super::{After, LinkCommand, Status};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const PING_INTERVAL: Duration = Duration::from_secs(5);
const RETRY_DELAY: Duration = Duration::from_secs(3);

enum End {
    Lost,
    Stopped(Status),
    Done
}

pub fn spawn(app: AppHandle, id: u64, url: String, auth: Option<String>) -> mpsc::UnboundedSender<LinkCommand> {
    let (tx, rx) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(run(app, id, url, auth, rx));
    tx
}

async fn run(app: AppHandle, id: u64, url: String, auth: Option<String>, mut commands: mpsc::UnboundedReceiver<LinkCommand>) {
    let url = format!("{}/host", url.trim_end_matches('/'));

    loop {
        match session(&app, id, &url, auth.as_deref(), &mut commands).await {
            End::Done => return,
            End::Stopped(status) => {
                super::set_status(&app, id, status);
                return wait_for_shutdown(&mut commands).await;
            }
            End::Lost => super::set_status(&app, id, Status::Closed)
        }

        let retry = sleep(RETRY_DELAY);
        tokio::pin!(retry);
        loop {
            tokio::select! {
                _ = &mut retry => break,
                command = commands.recv() => match command {
                    Some(LinkCommand::Shutdown) | None => return,
                    // Nobody to send it to. The webview sends its state again after auth-success.
                    Some(LinkCommand::Send(_)) => {}
                }
            }
        }
    }
}

async fn wait_for_shutdown(commands: &mut mpsc::UnboundedReceiver<LinkCommand>) {
    while let Some(command) = commands.recv().await {
        if matches!(command, LinkCommand::Shutdown) {
            return;
        }
    }
}

async fn session(
    app: &AppHandle,
    id: u64,
    url: &str,
    auth: Option<&str>,
    commands: &mut mpsc::UnboundedReceiver<LinkCommand>
) -> End {
    super::set_status(app, id, Status::Connecting);

    let mut socket = match crate::net::connect(url, CONNECT_TIMEOUT).await {
        Ok(socket) => socket,
        Err(err) => {
            tracing::debug!("Share server {url} unreachable: {err}");
            return End::Lost;
        }
    };

    tracing::info!("Connected to the share server at {url}");
    super::set_status(app, id, Status::Open);

    // Signed out: the socket stays open without it, and connecting again with a token sends it.
    if let Some(auth) = auth {
        super::debug_out(app, auth);
        if socket.send(Message::text(auth)).await.is_err() {
            return End::Lost;
        }
    }

    let mut ticker = interval_at(Instant::now() + PING_INTERVAL, PING_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    // A ping is out and nothing has come back since.
    let mut waiting = false;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if waiting {
                    tracing::info!("Share server stopped answering");
                    return End::Lost;
                }
                if socket.send(Message::text("ping")).await.is_err() {
                    return End::Lost;
                }
                waiting = true;
            }
            message = socket.next() => {
                waiting = false;
                match message {
                    Some(Ok(Message::Text(text))) => {
                        if text.as_str() == "pong" {
                            continue;
                        }
                        if let After::Stop(status) = super::on_message(app, id, text.to_string()) {
                            let _ = socket.close(None).await;
                            return End::Stopped(status);
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => return End::Lost,
                    Some(Ok(_)) => {}
                }
            }
            command = commands.recv() => match command {
                Some(LinkCommand::Send(text)) => {
                    if socket.send(Message::text(text)).await.is_err() {
                        return End::Lost;
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
