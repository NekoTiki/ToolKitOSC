//! One task per link to the OWO app while the add-on is on: ping it every 3 s (broadcast, or the
//! typed address) until it answers, identify, then send what the tick hands over to wherever it
//! answered from. `OWO_Close` goes back to pinging.
use std::net::SocketAddr;
use std::time::Duration;

use tauri::AppHandle;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time::interval;

use super::protocol::{self, Packet, Reply};
use super::{LinkCommand, Status};

const PING_EVERY: Duration = Duration::from_secs(3);

pub fn spawn(app: AppHandle, id: u64, target: String) -> mpsc::UnboundedSender<LinkCommand> {
    let (tx, rx) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(run(app, id, target, rx));
    tx
}

async fn send(socket: &UdpSocket, packet: &Packet, to: SocketAddr) {
    if let Err(err) = socket.send_to(packet.text().as_bytes(), to).await {
        tracing::debug!("Couldn't send to the OWO app at {to}: {err}");
    }
}

async fn run(app: AppHandle, id: u64, target: String, mut commands: mpsc::UnboundedReceiver<LinkCommand>) {
    let socket = match UdpSocket::bind(("0.0.0.0", 0)).await {
        Ok(socket) => socket,
        Err(err) => {
            tracing::error!("Couldn't open a socket for the OWO app: {err}");
            return;
        }
    };
    if let Err(err) = socket.set_broadcast(true) {
        tracing::warn!("Can't broadcast to find the OWO app: {err}");
    }
    let mut peer: Option<SocketAddr> = None;
    let mut ping = interval(PING_EVERY);
    let mut buf = [0u8; 1024];
    loop {
        tokio::select! {
            _ = ping.tick(), if peer.is_none() => {
                match tokio::net::lookup_host(&target).await.map(|mut found| found.next()) {
                    Ok(Some(to)) => send(&socket, &Packet::Ping, to).await,
                    _ => tracing::debug!("Can't find the OWO app's address {target}")
                }
            }
            received = socket.recv_from(&mut buf) => {
                // Errors are the ICMP answers to pings nobody heard (Windows reports them here).
                let Ok((count, from)) = received else { continue };
                match protocol::reply(&String::from_utf8_lossy(&buf[..count])) {
                    Some(Reply::Okay) if peer.is_none() => send(&socket, &Packet::Auth, from).await,
                    Some(Reply::Pong) if peer.is_none() => {
                        tracing::info!("Connected to the OWO app at {from}");
                        peer = Some(from);
                        super::set_status(&app, id, Status::Connected, peer);
                    }
                    Some(Reply::Close) if peer == Some(from) => {
                        tracing::info!("The OWO app closed the connection");
                        peer = None;
                        super::set_status(&app, id, Status::Connecting, None);
                    }
                    _ => {}
                }
            }
            command = commands.recv() => match command {
                Some(LinkCommand::Send(packet)) => {
                    if let Some(to) = peer {
                        send(&socket, &packet, to).await;
                    }
                }
                Some(LinkCommand::Shutdown) | None => {
                    if let Some(to) = peer {
                        send(&socket, &Packet::Stop, to).await;
                        send(&socket, &Packet::GameUnavailable, to).await;
                    }
                    return;
                }
            }
        }
    }
}

/// Where to ping: the typed address (`ip` or `ip:port`), or everyone on the network.
pub fn target(address: Option<&str>) -> String {
    let Some(address) = address.map(str::trim).filter(|address| !address.is_empty()) else {
        return format!("255.255.255.255:{}", protocol::PORT);
    };
    if let Ok(ip) = address.parse::<std::net::IpAddr>() {
        return SocketAddr::new(ip, protocol::PORT).to_string();
    }
    let has_port = address.parse::<SocketAddr>().is_ok() || address.rsplit_once(':').is_some_and(|(_, port)| port.parse::<u16>().is_ok());
    if has_port { address.to_owned() } else { format!("{address}:{}", protocol::PORT) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pings_the_typed_address_or_everyone() {
        assert_eq!(target(None), "255.255.255.255:54020");
        assert_eq!(target(Some("  ")), "255.255.255.255:54020");
        assert_eq!(target(Some("192.168.1.42")), "192.168.1.42:54020");
        assert_eq!(target(Some("192.168.1.42:6000")), "192.168.1.42:6000");
        assert_eq!(target(Some("pixel.local")), "pixel.local:54020");
        assert_eq!(target(Some("fe80::1")), "[fe80::1]:54020");
    }
}
