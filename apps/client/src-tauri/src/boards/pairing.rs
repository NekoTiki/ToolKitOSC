//! Adding a board: `pair`, wait for someone to press BOOT on it, then `hello` with the new token on
//! the same socket to learn its id and outputs. One pairing at a time.
use std::time::Duration;

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

use super::link::{self, Socket};
use super::protocol::{self, Incoming, Outgoing, Welcome};
use super::store::BoardRecord;
use super::{boards, BoardView};

/// The board's own window is 30 s; this leaves room for the network on top.
const PAIR_TIMEOUT: Duration = Duration::from_secs(40);
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
enum PairingEvent {
    Connecting {
        name: String
    },
    Waiting {
        name: String,
        #[serde(rename = "timeoutMs")]
        timeout_ms: u32
    },
    Paired {
        id: String,
        name: String
    },
    Failed {
        reason: String
    },
    Cancelled
}

fn emit(app: &AppHandle, event: PairingEvent) {
    let _ = app.emit("board-pairing", event);
}

/// Where to reach the board: a found board's id, or an address typed by the user
/// (`192.168.1.40`, `tkosc-a1b2c3.local`, optionally with `:port`).
pub enum Target {
    Found(String),
    Address(String)
}

fn resolve(app: &AppHandle, target: &Target) -> Result<(String, u16, String), String> {
    match target {
        Target::Found(id) => super::found_address(app, id).ok_or_else(|| "That board isn't on the network any more.".to_string()),
        Target::Address(address) => {
            let address = address.trim();
            if address.is_empty() {
                return Err("Enter the board's address.".into());
            }
            // host:port, but leave bare IPv6 addresses alone.
            match address.rsplit_once(':') {
                Some((host, port)) if !host.contains(':') => {
                    let port = port.parse().map_err(|_| format!("\"{port}\" isn't a port number."))?;
                    Ok((host.to_string(), port, host.to_string()))
                }
                _ => Ok((address.to_string(), protocol::DEFAULT_PORT, address.to_string()))
            }
        }
    }
}

pub async fn pair(app: AppHandle, target: Target) -> Result<BoardView, String> {
    let cancelled = {
        let mut inner = boards(&app).inner.lock().unwrap();
        if inner.pairing.is_some() {
            return Err("Another board is being added. Finish or cancel that first.".into());
        }
        let (tx, rx) = oneshot::channel();
        inner.pairing = Some(tx);
        rx
    };

    let result = tokio::select! {
        result = run(&app, &target) => result,
        _ = cancelled => Err(String::new())
    };
    boards(&app).inner.lock().unwrap().pairing = None;

    match result {
        Ok(view) => {
            emit(&app, PairingEvent::Paired { id: view.id.clone(), name: view.name.clone() });
            Ok(view)
        }
        Err(reason) if reason.is_empty() => {
            emit(&app, PairingEvent::Cancelled);
            Err("Cancelled".into())
        }
        Err(reason) => {
            tracing::warn!("Adding an ESP32 board failed: {reason}");
            emit(&app, PairingEvent::Failed { reason: reason.clone() });
            Err(reason)
        }
    }
}

async fn run(app: &AppHandle, target: &Target) -> Result<BoardView, String> {
    let (host, port, name) = resolve(app, target)?;
    emit(app, PairingEvent::Connecting { name: name.clone() });

    let mut socket = link::connect(&host, port)
        .await
        .map_err(|err| format!("Couldn't reach the board at {host}:{port} ({err})."))?;

    link::send(&mut socket, &Outgoing::Pair).await?;

    let token = timeout(PAIR_TIMEOUT, async {
        loop {
            match next(&mut socket).await? {
                Incoming::Pairing { timeout_ms } => {
                    emit(app, PairingEvent::Waiting { name: name.clone(), timeout_ms });
                }
                Incoming::Paired { token } => return Ok(token),
                Incoming::Err { code, .. } if code == "pair-timeout" => {
                    return Err("Nobody pressed the BOOT button in time.".to_string());
                }
                _ => {}
            }
        }
    })
    .await
    .map_err(|_| "The board didn't answer.".to_string())??;

    link::send(&mut socket, &Outgoing::Hello { proto: protocol::PROTO, token: &token }).await?;

    let welcome: Welcome = timeout(REPLY_TIMEOUT, async {
        loop {
            match next(&mut socket).await? {
                Incoming::Welcome(welcome) => return Ok(welcome),
                Incoming::Err { code, .. } if code == "bad-proto" => {
                    return Err("This board's firmware is for another version of ToolKitOSC. Update it.".to_string());
                }
                _ => {}
            }
        }
    })
    .await
    .map_err(|_| "The board didn't answer after pairing.".to_string())??;

    // The board's link task opens its own session right after.
    let _ = socket.close(None).await;

    let record = BoardRecord {
        id: welcome.id.clone(),
        name: welcome.name.clone(),
        token,
        host,
        port,
        fw: Some(welcome.fw.clone()),
        chip: Some(welcome.chip.clone()),
        board: Some(welcome.board.clone()),
        outputs: welcome.outputs.clone()
    };
    tracing::info!("Added ESP32 board {} ({})", record.name, record.id);
    super::add_record(app, record);

    super::snapshot(app)
        .boards
        .into_iter()
        .find(|b| b.id == welcome.id)
        .ok_or_else(|| "The board was added but isn't in the list.".to_string())
}

/// The next message the board sends, or why the socket ended.
async fn next(socket: &mut Socket) -> Result<Incoming, String> {
    loop {
        match socket.next().await {
            Some(Ok(Message::Text(text))) => {
                if let Some(message) = protocol::decode(&text) {
                    return Ok(message);
                }
            }
            Some(Ok(Message::Close(frame))) => {
                return Err(match link::close_code(&frame) {
                    Some(protocol::CLOSE_BAD_PROTO) => "This board's firmware is for another version of ToolKitOSC.".into(),
                    _ => "The board closed the connection.".into()
                });
            }
            Some(Ok(_)) => {}
            Some(Err(err)) => return Err(format!("The connection to the board broke ({err}).")),
            None => return Err("The board closed the connection.".into())
        }
    }
}
