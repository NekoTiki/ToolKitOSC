use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tauri::tray::TrayIcon;
use tokio::net::UdpSocket;
use vrchat_osc::VRChatOSC;

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
    /// VRChat's real OSC address, once `osc::oscquery` discovers it via mDNS - overrides the fixed
    /// send port 9000 in `commands::send_osc_message` when present. `None` until discovery
    /// succeeds (or forever, on a network where mDNS doesn't work), in which case sending falls
    /// back to the fixed port unchanged.
    pub oscquery_target: Mutex<Option<SocketAddr>>,
    /// VRChat's OSCQuery HTTP address (distinct from `oscquery_target`, which is the OSC UDP
    /// address) - lets `commands::force_pull_parameters` issue an on-demand GET without waiting
    /// for a new discovery event.
    pub oscquery_http_target: Mutex<Option<SocketAddr>>,
    /// Kept alive for the app's lifetime once `osc::oscquery::run` creates it - see that module.
    pub oscquery: Mutex<Option<Arc<VRChatOSC>>>,
    /// The task polling VRChat's OSCQuery tree for avatar changes (see `osc::oscquery`) - aborted
    /// and replaced whenever VRChat's OSCQuery service is rediscovered at a new address, so a
    /// stale poll never keeps running against an address that's no longer VRChat.
    pub avatar_poll_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Mirrors the frontend's "keep running in the tray" setting (persisted in the webview's own
    /// localStorage, out of reach of the window's native close handler) - kept in sync via
    /// `commands::set_minimize_to_tray`, which the frontend calls on load and on every toggle.
    pub minimize_to_tray: AtomicBool,
    /// The tray icon built once in `lib.rs::setup_tray`. Kept here so `commands::set_minimize_to_tray`
    /// can show/hide it in step with the setting - the icon only earns its keep as a way back into
    /// a window that's hidden rather than closed, so it has no reason to exist while the setting is off.
    pub tray_icon: Mutex<Option<TrayIcon>>
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Mutex::new(HashMap::new()),
            avatar_details: Mutex::new(None),
            cache_throttle: CacheThrottle::default(),
            sender_socket: Mutex::new(None),
            oscquery_target: Mutex::new(None),
            oscquery_http_target: Mutex::new(None),
            oscquery: Mutex::new(None),
            avatar_poll_task: Mutex::new(None),
            minimize_to_tray: AtomicBool::new(false),
            tray_icon: Mutex::new(None)
        }
    }
}
