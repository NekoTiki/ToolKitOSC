//! The panel's end of the localhost WebSocket to the app, on its own thread: what the app sends
//! comes out of `states`, what you press goes into `actions`. Both close when the app goes away.
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use tokio_tungstenite::tungstenite::{self, Message};

use super::protocol::{State, ToApp};

pub struct Link {
    pub states: Receiver<State>,
    pub actions: Sender<ToApp>
}

pub fn connect(port: u16, token: String) -> Result<Link, String> {
    let stream = TcpStream::connect(("127.0.0.1", port)).map_err(|e| format!("Couldn't reach the app: {e}"))?;
    let (mut socket, _) = tungstenite::client(format!("ws://127.0.0.1:{port}/"), stream).map_err(|e| e.to_string())?;
    // Short reads, so the thread also gets to send.
    socket.get_ref().set_read_timeout(Some(Duration::from_millis(20))).map_err(|e| e.to_string())?;
    socket.send(Message::text(serde_json::to_string(&ToApp::Hello { token }).unwrap())).map_err(|e| e.to_string())?;

    let (state_tx, states) = mpsc::channel();
    let (actions, action_rx) = mpsc::channel::<ToApp>();
    std::thread::spawn(move || loop {
        match socket.read() {
            Ok(Message::Text(text)) => match serde_json::from_str::<State>(&text) {
                Ok(state) => {
                    if state_tx.send(state).is_err() {
                        return;
                    }
                }
                Err(err) => tracing::warn!("Panel: bad state from the app: {err}"),
            },
            Ok(Message::Close(_)) => return,
            Ok(_) => {}
            Err(tungstenite::Error::Io(err)) if matches!(err.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(err) => {
                tracing::info!("Panel: the link to the app closed: {err}");
                return;
            }
        }
        while let Ok(action) = action_rx.try_recv() {
            if socket.send(Message::text(serde_json::to_string(&action).unwrap())).is_err() {
                return;
            }
        }
    });
    Ok(Link { states, actions })
}
