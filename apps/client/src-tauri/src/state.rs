use std::collections::HashMap;
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
    pub sender_socket: Mutex<Option<Arc<UdpSocket>>>
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Mutex::new(HashMap::new()),
            avatar_details: Mutex::new(None),
            cache_throttle: CacheThrottle::default(),
            sender_socket: Mutex::new(None)
        }
    }
}
