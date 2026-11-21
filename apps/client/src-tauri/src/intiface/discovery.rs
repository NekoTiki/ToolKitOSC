//! Browses for Intiface engines that announce themselves over mDNS (Intiface Central's "broadcast
//! server via mDNS", on the phone apps too). They announce `_intiface_engine._tcp` under a random
//! name that changes on every start, so a device is known by its hostname. Like the ESP32 boards'
//! discovery, this doesn't cross VPNs or networks that block multicast: adding a server by address
//! still works there.
use std::net::{IpAddr, UdpSocket};

use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent};
use tauri::AppHandle;

const SERVICE_TYPE: &str = "_intiface_engine._tcp.local.";

/// An Intiface engine announced on the network right now.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    /// Without `.local`, e.g. "pixel-7".
    pub hostname: String,
    pub host: String,
    pub port: u16,
    /// The announcement's full name, to match its removal.
    #[serde(skip)]
    pub fullname: String
}

/// Starts browsing. Returns the daemon, so shutting it down ends the browse.
pub fn start(app: AppHandle) -> Option<ServiceDaemon> {
    let daemon = match ServiceDaemon::new() {
        Ok(daemon) => daemon,
        Err(err) => {
            tracing::error!("mDNS for Intiface couldn't start, servers can only be added by address: {err}");
            return None;
        }
    };
    let events = match daemon.browse(SERVICE_TYPE) {
        Ok(events) => events,
        Err(err) => {
            tracing::error!("Couldn't browse for Intiface devices: {err}");
            return None;
        }
    };
    tracing::info!("Browsing for Intiface devices");

    tauri::async_runtime::spawn(async move {
        while let Ok(event) = events.recv_async().await {
            match event {
                ServiceEvent::ServiceResolved(service) => {
                    if let Some(found) = parse(&service) {
                        tracing::debug!("Found Intiface on {} at {}:{}", found.hostname, found.host, found.port);
                        super::found(&app, found);
                    }
                }
                ServiceEvent::ServiceRemoved(_, fullname) => super::lost(&app, &fullname),
                _ => {}
            }
        }
        tracing::info!("Stopped browsing for Intiface devices");
    });
    Some(daemon)
}

fn parse(service: &ResolvedService) -> Option<Found> {
    let ips: Vec<IpAddr> = service.addresses.iter().map(|a| a.to_ip_addr()).filter(|ip| !ip.is_loopback()).collect();
    // This PC's own Intiface Central: the "this PC" server already covers it.
    if ips.iter().any(|ip| is_this_pc(*ip)) {
        return None;
    }
    // IPv4 first: phones on Wi-Fi are reached most reliably that way.
    let host = ips.iter().min_by_key(|ip| !matches!(ip, IpAddr::V4(_)))?.to_string();
    Some(Found { hostname: hostname(&service.host)?, host, port: service.port, fullname: service.fullname.clone() })
}

/// "Pixel-7.local." → "pixel-7".
fn hostname(host: &str) -> Option<String> {
    let name = host.trim_end_matches('.').trim_end_matches(".local").to_lowercase();
    (!name.is_empty()).then_some(name)
}

/// Only this machine's own addresses can be bound to.
fn is_this_pc(ip: IpAddr) -> bool {
    UdpSocket::bind((ip, 0)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostnames_lose_their_local_suffix() {
        assert_eq!(hostname("Pixel-7.local.").as_deref(), Some("pixel-7"));
        assert_eq!(hostname("desk.local").as_deref(), Some("desk"));
        assert_eq!(hostname(".local."), None);
    }

    #[test]
    fn this_pc_is_recognised() {
        assert!(is_this_pc("127.0.0.1".parse().unwrap()));
        // TEST-NET-3, never assigned to a machine.
        assert!(!is_this_pc("203.0.113.7".parse().unwrap()));
    }
}
