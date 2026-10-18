//! The OWO tester: a pretend OWO app for development builds, so the whole chain from VRChat to the
//! socket runs without a suit. It listens next to the real app's port, answers `ping` and `AUTH`
//! the way the app does, and keeps the last 200 packets it was sent for Settings › OWO.
//!
//! Development builds only, three times over: this module is `#[cfg(debug_assertions)]`,
//! `set_simulated` refuses in a release, and Settings › OWO only shows its switch when
//! `import.meta.env.DEV`.
use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use tokio::net::UdpSocket;

use super::model::Muscle;
use super::protocol::{Packet, PORT as OWO_PORT};

pub const PORT: u16 = OWO_PORT + 1;
const KEEP: usize = 200;

/// One packet the tester got.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// Since the tester started.
    pub at_ms: u64,
    pub text: String,
    /// What each muscle feels, 0–100: the sensation's intensity times the muscle's share.
    pub muscles: BTreeMap<Muscle, u8>,
    /// How long it lasts.
    pub ms: u32,
    pub stop: bool
}

static LOG: Mutex<VecDeque<Entry>> = Mutex::new(VecDeque::new());

pub fn log() -> Vec<Entry> {
    LOG.lock().unwrap().iter().cloned().collect()
}

fn entry(started: Instant, text: &str) -> Entry {
    let mut entry = Entry { at_ms: started.elapsed().as_millis() as u64, text: text.to_owned(), muscles: BTreeMap::new(), ms: 0, stop: false };
    match Packet::parse(text) {
        Some(Packet::Sensation { micro, muscles }) => {
            entry.ms = u32::from(micro.tenths) * 100;
            entry.muscles = muscles.into_iter().map(|(muscle, percent)| (muscle, (u32::from(micro.intensity) * u32::from(percent) / 100) as u8)).collect();
        }
        Some(Packet::Stop | Packet::GameUnavailable) => entry.stop = true,
        _ => {}
    }
    entry
}

/// Starts listening, once. Errors (the port is taken) are logged, and the link then never
/// connects.
pub fn start() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        tauri::async_runtime::spawn(async {
            let socket = match UdpSocket::bind(("127.0.0.1", PORT)).await {
                Ok(socket) => socket,
                Err(err) => {
                    tracing::error!("The OWO tester couldn't listen on {PORT}: {err}");
                    return;
                }
            };
            tracing::info!("OWO tester on port {PORT}");
            let started = Instant::now();
            let mut buf = [0u8; 2048];
            loop {
                let Ok((count, from)) = socket.recv_from(&mut buf).await else { continue };
                let text = String::from_utf8_lossy(&buf[..count]).into_owned();
                let answer = match Packet::parse(&text) {
                    Some(Packet::Ping) => Some("okay"),
                    Some(Packet::Auth) => Some("pong"),
                    _ => None
                };
                if let Some(answer) = answer {
                    let _ = socket.send_to(answer.as_bytes(), from).await;
                    continue;
                }
                let mut log = LOG.lock().unwrap();
                log.push_back(entry(started, &text));
                while log.len() > KEEP {
                    log.pop_front();
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_each_muscle_feels() {
        let entry = entry(Instant::now(), "0*SENSATION*100,3,60,0,0,0,TKOSC|1%100,9%50");
        assert_eq!(entry.ms, 300);
        assert_eq!(entry.muscles, BTreeMap::from([(Muscle::PectoralL, 60), (Muscle::LumbarL, 30)]));
        assert!(super::entry(Instant::now(), "0*STOP").stop);
    }
}
