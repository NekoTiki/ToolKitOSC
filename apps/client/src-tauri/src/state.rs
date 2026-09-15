use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;

use crate::osc::cache::CacheThrottle;
use crate::types::{AvatarDetails, OscMessage};

pub struct AppState {
    /// Last-known value of every OSC address for the current avatar. Mirrors the original's
    /// module-level `OSC_MESSAGES` Map.
    pub messages: Mutex<HashMap<String, OscMessage>>,
    pub avatar_details: Mutex<Option<AvatarDetails>>,
    pub cache_throttle: CacheThrottle,
    /// The socket already `.connect()`-ed to VRChat's inbound OSC port (127.0.0.1:9000). Set
    /// once at startup by `osc::udp::run`; `commands::send_osc_message` sends directly on it.
    pub sender_socket: Mutex<Option<Arc<UdpSocket>>>,
    /// Mirrors the frontend's "keep running in the tray" setting (persisted in the webview's own
    /// localStorage, out of reach of the window's native close handler) - kept in sync via
    /// `commands::set_minimize_to_tray`, which the frontend calls on load and on every toggle.
    pub minimize_to_tray: AtomicBool
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Mutex::new(HashMap::new()),
            avatar_details: Mutex::new(None),
            cache_throttle: CacheThrottle::default(),
            sender_socket: Mutex::new(None),
            minimize_to_tray: AtomicBool::new(false)
        }
    }
}
