//! WebSocket helpers shared by the links to the outside world (ESP32 boards, the share server).
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Opens a `ws://` or `wss://` connection, giving up after `limit`.
pub async fn connect(url: &str, limit: Duration) -> Result<Socket, String> {
    match timeout(limit, tokio_tungstenite::connect_async(url)).await {
        Ok(Ok((socket, _))) => Ok(socket),
        Ok(Err(err)) => Err(err.to_string()),
        Err(_) => Err("timed out".into())
    }
}

/// The close code of a close frame, if it has one.
pub fn close_code(frame: &Option<CloseFrame>) -> Option<u16> {
    frame.as_ref().map(|f| u16::from(f.code))
}
