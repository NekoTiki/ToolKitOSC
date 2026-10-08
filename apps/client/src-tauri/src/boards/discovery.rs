//! Browses for `_tkosc._tcp` boards. Separate from the mDNS stack inside `vrchat_osc` (see
//! `osc::oscquery`), which only knows VRChat's own service types. Like OSCQuery, this doesn't work
//! across VPNs or on networks that block multicast: that's what "Add by address" is for.
use std::net::IpAddr;

use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent};
use tauri::AppHandle;

use super::FoundBoard;

const SERVICE_TYPE: &str = "_tkosc._tcp.local.";

pub fn start(app: AppHandle) {
    let daemon = match ServiceDaemon::new() {
        Ok(daemon) => daemon,
        Err(err) => {
            tracing::error!("mDNS for ESP32 boards couldn't start, boards can only be added by address: {err}");
            return;
        }
    };
    let events = match daemon.browse(SERVICE_TYPE) {
        Ok(events) => events,
        Err(err) => {
            tracing::error!("Couldn't browse for ESP32 boards: {err}");
            return;
        }
    };

    tauri::async_runtime::spawn(async move {
        // Owned by the task so it lives as long as the browse does.
        let _daemon = daemon;

        while let Ok(event) = events.recv_async().await {
            match event {
                ServiceEvent::ServiceResolved(service) => {
                    if let Some(found) = parse(&service) {
                        tracing::debug!("Found ESP32 board {} ({}) at {}:{}", found.name, found.id, found.host, found.port);
                        super::found_update(&app, found);
                    }
                }
                ServiceEvent::ServiceRemoved(_, fullname) => super::found_remove(&app, &fullname),
                _ => {}
            }
        }
    });
}

fn parse(service: &ResolvedService) -> Option<FoundBoard> {
    let txt = &service.txt_properties;
    let id = txt.get_property_val_str("id")?.to_string();

    // IPv4 first: the board's WebSocket server only listens on IPv4.
    let host = service
        .addresses
        .iter()
        .map(|a| a.to_ip_addr())
        .filter(|ip| !ip.is_loopback())
        .min_by_key(|ip| !matches!(ip, IpAddr::V4(_)))?
        .to_string();

    Some(FoundBoard {
        name: txt.get_property_val_str("name").unwrap_or(&id).to_string(),
        host,
        port: service.port,
        fw: txt.get_property_val_str("fw").map(str::to_string),
        proto: txt.get_property_val_str("proto").and_then(|v| v.parse().ok()),
        paired: txt.get_property_val_str("paired") == Some("1"),
        added: false,
        fullname: service.fullname.clone(),
        id
    })
}
