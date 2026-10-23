//! The localhost WebSocket the panel connects to. It only listens on 127.0.0.1, and only a
//! connection that opens with the one-time token passed on the panel's command line is answered.
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

use super::{overlay, refresh};
use crate::wrist::protocol::{State, ToApp};

/// While the panel is connected, its state is checked this often: toy levels, the leash and touch
/// change without an event of their own, and the count of the last minute goes down by itself.
const LIVE_EVERY: Duration = Duration::from_millis(250);

/// Opens the socket. Called when the add-on starts.
pub async fn listen(app: AppHandle) {
    let listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!("Panel: couldn't open the local socket: {err}");
            return;
        }
    };
    let port = listener.local_addr().map(|addr| addr.port()).unwrap_or_default();
    let token = format!("{:032x}", rand::random::<u128>());
    {
        let mut panel = overlay(&app).panel.lock().unwrap();
        panel.listening = Some((port, token.clone()));
        panel.wake = Some(watch::channel(()).0);
    }
    while let Ok((stream, _)) = listener.accept().await {
        let app = app.clone();
        let token = token.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(err) = serve(&app, stream, &token).await {
                tracing::debug!("Panel link closed: {err}");
            }
        });
    }
}

async fn serve(app: &AppHandle, stream: tokio::net::TcpStream, token: &str) -> Result<(), String> {
    let socket = tokio_tungstenite::accept_async(stream).await.map_err(|e| e.to_string())?;
    let (mut write, mut read) = socket.split();
    let hello = tokio::time::timeout(Duration::from_secs(5), read.next()).await.map_err(|_| "no hello")?;
    let Some(Ok(Message::Text(text))) = hello else {
        return Err("no hello".into());
    };
    if !matches!(serde_json::from_str(&text), Ok(ToApp::Hello { token: sent }) if sent == token) {
        return Err("wrong token".into());
    }
    let mut wake = overlay(app).panel.lock().unwrap().wake.as_ref().ok_or("the add-on is off")?.subscribe();
    let mut live = tokio::time::interval(LIVE_EVERY);
    let mut sent: Option<State> = None;
    loop {
        tokio::select! {
            _ = live.tick() => {}
            woken = wake.changed() => {
                woken.map_err(|_| "the add-on stopped")?;
                wake.borrow_and_update();
            }
            message = read.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => match serde_json::from_str::<ToApp>(&text) {
                        Ok(action) => act(app, action),
                        Err(err) => tracing::warn!("Panel: unknown message: {err}")
                    },
                    Some(Ok(_)) => {}
                    _ => return Ok(())
                }
                continue;
            }
        }
        let state = super::state::build(app);
        if sent.as_ref() != Some(&state) {
            write.send(Message::text(serde_json::to_string(&state).unwrap())).await.map_err(|e| e.to_string())?;
            sent = Some(state);
        }
    }
}

/// Something pressed on the panel.
fn act(app: &AppHandle, action: ToApp) {
    tracing::info!("Panel: {action:?}");
    match &action {
        ToApp::Control { group_id, control_id, level, pick } => {
            super::pinned::press(app, group_id, control_id, *level, *pick);
            refresh(app);
        }
        ToApp::Hud { on } => super::super::set_hud(app, *on),
        // The connection picks it up once its server starts.
        ToApp::LaunchIntiface => {
            if let Err(err) = crate::intiface::launch_now() {
                tracing::warn!("Panel: couldn't start Intiface Central: {err}");
            }
        }
        ToApp::Stop => crate::setup::stop::stop_now(app),
        ToApp::Resume => crate::setup::stop::resume(app),
        // A profile deleted since is ignored.
        ToApp::Profile { id } => {
            if let Err(err) = crate::setup::switch_profile(app, id.clone()) {
                tracing::debug!("Panel: couldn't switch the profile: {err}");
            }
        }
        ToApp::Hello { .. } => {}
    }
}
