//! The OpenVR calls the wrist panel needs, from SteamVR's own `openvr_api.dll` loaded at runtime,
//! the same way `crate::steamvr` does. The function tables mirror `openvr_capi.h` for
//! `IVRSystem_026`, `IVROverlay_028` and `IVRInput_011`: only the members called here have real signatures, the
//! rest are pointer-sized filler that keeps the offsets right (counted from the header, where each
//! member's index is its line in the struct).
use std::ffi::{c_char, c_void, CString};

use libloading::{Library, Symbol};

const APP_TYPE_OVERLAY: i32 = 2; // EVRApplicationType_VRApplication_Overlay
pub const ORIGIN_STANDING: i32 = 1; // ETrackingUniverseOrigin_TrackingUniverseStanding
pub const ROLE_LEFT: i32 = 1; // ETrackedControllerRole_LeftHand
pub const ROLE_RIGHT: i32 = 2;
pub const HMD: u32 = 0; // k_unTrackedDeviceIndex_Hmd
pub const INVALID_DEVICE: u32 = u32::MAX;
const MAX_DEVICES: usize = 64; // k_unMaxTrackedDeviceCount
pub const EVENT_QUIT: u32 = 700; // EVREventType_VREvent_Quit

/// A rigid transform: rotation in the first three columns, translation in the last.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Matrix34 {
    pub m: [[f32; 4]; 3]
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TrackedDevicePose {
    pub device_to_absolute: Matrix34,
    velocity: [f32; 3],
    angular_velocity: [f32; 3],
    tracking_result: i32,
    pub valid: bool,
    pub connected: bool
}

#[repr(C)]
#[derive(Default)]
struct DigitalActionData {
    active: bool,
    origin: u64,
    state: bool,
    changed: bool,
    update_time: f32
}

#[repr(C)]
#[derive(Default)]
struct PoseActionData {
    active: bool,
    origin: u64,
    pose: TrackedDevicePose
}

#[repr(C)]
struct ActiveActionSet {
    set: u64,
    restricted_to_device: u64,
    secondary: u64,
    padding: u32,
    priority: i32
}

/// `VREvent_t`: three 32-bit fields, then the data union, whose largest member is 6 × u64.
#[repr(C)]
pub struct Event {
    pub event_type: u32,
    device: u32,
    age: f32,
    data: [u64; 6]
}

#[repr(C)]
struct SystemFnTable {
    _a: [*const c_void; 12],
    get_device_to_absolute_tracking_pose: unsafe extern "system" fn(i32, f32, *mut TrackedDevicePose, u32),
    _b: [*const c_void; 5],
    get_tracked_device_index_for_controller_role: unsafe extern "system" fn(i32) -> u32,
    _c: [*const c_void; 11],
    poll_next_event: unsafe extern "system" fn(*mut Event, u32) -> bool,
    _d: [*const c_void; 16],
    acknowledge_quit_exiting: unsafe extern "system" fn()
}

#[repr(C)]
struct OverlayFnTable {
    _a: [*const c_void; 1],
    create_overlay: unsafe extern "system" fn(*mut c_char, *mut c_char, *mut u64) -> i32,
    _b: [*const c_void; 1],
    destroy_overlay: unsafe extern "system" fn(u64) -> i32,
    _c: [*const c_void; 10],
    set_overlay_color: unsafe extern "system" fn(u64, f32, f32, f32) -> i32,
    _c2: [*const c_void; 1],
    set_overlay_alpha: unsafe extern "system" fn(u64, f32) -> i32,
    _d: [*const c_void; 3],
    set_overlay_sort_order: unsafe extern "system" fn(u64, u32) -> i32,
    _d2: [*const c_void; 1],
    set_overlay_width_in_meters: unsafe extern "system" fn(u64, f32) -> i32,
    _e: [*const c_void; 12],
    set_overlay_transform_tracked_device_relative: unsafe extern "system" fn(u64, u32, *const Matrix34) -> i32,
    _f: [*const c_void; 7],
    show_overlay: unsafe extern "system" fn(u64) -> i32,
    hide_overlay: unsafe extern "system" fn(u64) -> i32,
    _g: [*const c_void; 17],
    set_overlay_raw: unsafe extern "system" fn(u64, *mut c_void, u32, u32, u32) -> i32
}

#[repr(C)]
struct InputFnTable {
    set_action_manifest_path: unsafe extern "system" fn(*mut c_char) -> i32,
    get_action_set_handle: unsafe extern "system" fn(*mut c_char, *mut u64) -> i32,
    get_action_handle: unsafe extern "system" fn(*mut c_char, *mut u64) -> i32,
    get_input_source_handle: unsafe extern "system" fn(*mut c_char, *mut u64) -> i32,
    update_action_state: unsafe extern "system" fn(*mut ActiveActionSet, u32, u32) -> i32,
    get_digital_action_data: unsafe extern "system" fn(u64, *mut DigitalActionData, u32, u64) -> i32,
    _a: [*const c_void; 1],
    get_pose_action_data_relative_to_now: unsafe extern "system" fn(u64, i32, f32, *mut PoseActionData, u32, u64) -> i32
}

type VrInitInternalFn = unsafe extern "system" fn(*mut i32, i32) -> isize;
type VrShutdownInternalFn = unsafe extern "system" fn();
type VrGetGenericInterfaceFn = unsafe extern "system" fn(*const c_char, *mut i32) -> isize;

pub struct OpenVr {
    lib: Library,
    system: &'static SystemFnTable,
    overlay: &'static OverlayFnTable,
    input: &'static InputFnTable
}

/// Handles from the action manifest (see `super::input`).
pub struct Actions {
    set: u64,
    click: u64,
    pointer: u64,
    /// `/user/hand/left` and `/user/hand/right`.
    hands: [u64; 2]
}

impl OpenVr {
    /// Loads SteamVR's OpenVR library and starts as an overlay app. Fails when SteamVR isn't
    /// running.
    pub fn init() -> Result<Self, String> {
        let path = crate::steamvr::find_openvr_api().ok_or("SteamVR was not found on this system")?;
        unsafe {
            let lib = Library::new(&path).map_err(|e| format!("Failed to load the OpenVR library: {e}"))?;
            let init: Symbol<VrInitInternalFn> = lib.get(b"VR_InitInternal\0").map_err(|e| e.to_string())?;
            let mut error = 0i32;
            init(&mut error, APP_TYPE_OVERLAY);
            drop(init);
            if error != 0 {
                return Err(format!("OpenVR didn't start (error {error})"));
            }
            let system = interface::<SystemFnTable>(&lib, "IVRSystem_026")?;
            let overlay = interface::<OverlayFnTable>(&lib, "IVROverlay_028")?;
            let input = interface::<InputFnTable>(&lib, "IVRInput_011")?;
            Ok(Self { lib, system, overlay, input })
        }
    }

    pub fn poses(&self) -> [TrackedDevicePose; MAX_DEVICES] {
        let mut poses = [TrackedDevicePose::default(); MAX_DEVICES];
        unsafe { (self.system.get_device_to_absolute_tracking_pose)(ORIGIN_STANDING, 0.0, poses.as_mut_ptr(), MAX_DEVICES as u32) };
        poses
    }

    pub fn controller(&self, role: i32) -> u32 {
        unsafe { (self.system.get_tracked_device_index_for_controller_role)(role) }
    }

    /// Loads the action manifest at `path` and looks up its actions.
    pub fn actions(&self, path: &std::path::Path) -> Result<Actions, String> {
        let c = |text: &str| CString::new(text).unwrap();
        let path = c(&path.to_string_lossy());
        unsafe {
            input_check((self.input.set_action_manifest_path)(path.as_ptr().cast_mut()))?;
            let handle = |get: unsafe extern "system" fn(*mut c_char, *mut u64) -> i32, name: &str| -> Result<u64, String> {
                let name = c(name);
                let mut handle = 0u64;
                input_check(get(name.as_ptr().cast_mut(), &mut handle))?;
                Ok(handle)
            };
            Ok(Actions {
                set: handle(self.input.get_action_set_handle, "/actions/wrist")?,
                click: handle(self.input.get_action_handle, "/actions/wrist/in/click")?,
                pointer: handle(self.input.get_action_handle, "/actions/wrist/in/pointer")?,
                hands: [
                    handle(self.input.get_input_source_handle, "/user/hand/left")?,
                    handle(self.input.get_input_source_handle, "/user/hand/right")?
                ]
            })
        }
    }

    /// Reads the actions once a tick, before `click` and `pointer`.
    pub fn update_actions(&self, actions: &Actions) {
        let mut set = ActiveActionSet { set: actions.set, restricted_to_device: 0, secondary: 0, padding: 0, priority: 0 };
        unsafe { (self.input.update_action_state)(&mut set, std::mem::size_of::<ActiveActionSet>() as u32, 1) };
    }

    /// Whether the click (the trigger, by default) is held on that hand.
    pub fn click(&self, actions: &Actions, left: bool) -> bool {
        let mut data = DigitalActionData::default();
        let hand = actions.hands[usize::from(!left)];
        let error = unsafe { (self.input.get_digital_action_data)(actions.click, &mut data, std::mem::size_of::<DigitalActionData>() as u32, hand) };
        error == 0 && data.active && data.state
    }

    /// Where that hand points from, and which way: SteamVR's tip pose for the controller.
    pub fn pointer(&self, actions: &Actions, left: bool) -> Option<Matrix34> {
        let mut data = PoseActionData::default();
        let hand = actions.hands[usize::from(!left)];
        let size = std::mem::size_of::<PoseActionData>() as u32;
        let error = unsafe { (self.input.get_pose_action_data_relative_to_now)(actions.pointer, ORIGIN_STANDING, 0.0, &mut data, size, hand) };
        (error == 0 && data.active && data.pose.valid).then_some(data.pose.device_to_absolute)
    }

    /// Whether SteamVR asked us to quit. Every other event is dropped.
    pub fn quit_requested(&self) -> bool {
        let mut event: Event = unsafe { std::mem::zeroed() };
        while unsafe { (self.system.poll_next_event)(&mut event, std::mem::size_of::<Event>() as u32) } {
            if event.event_type == EVENT_QUIT {
                unsafe { (self.system.acknowledge_quit_exiting)() };
                return true;
            }
        }
        false
    }

    pub fn create_overlay(&self, key: &str, name: &str) -> Result<u64, String> {
        let key = CString::new(key).unwrap();
        let name = CString::new(name).unwrap();
        let mut handle = 0u64;
        check(unsafe { (self.overlay.create_overlay)(key.as_ptr().cast_mut(), name.as_ptr().cast_mut(), &mut handle) })?;
        Ok(handle)
    }

    pub fn destroy_overlay(&self, handle: u64) {
        unsafe { (self.overlay.destroy_overlay)(handle) };
    }

    pub fn set_alpha(&self, handle: u64, alpha: f32) {
        unsafe { (self.overlay.set_overlay_alpha)(handle, alpha) };
    }

    /// Tints the overlay; 1, 1, 1 shows it as drawn.
    pub fn set_color(&self, handle: u64, rgb: [f32; 3]) {
        unsafe { (self.overlay.set_overlay_color)(handle, rgb[0], rgb[1], rgb[2]) };
    }

    /// Higher draws on top of lower.
    pub fn set_sort_order(&self, handle: u64, order: u32) {
        unsafe { (self.overlay.set_overlay_sort_order)(handle, order) };
    }

    pub fn set_width(&self, handle: u64, meters: f32) {
        unsafe { (self.overlay.set_overlay_width_in_meters)(handle, meters) };
    }

    pub fn attach(&self, handle: u64, device: u32, transform: &Matrix34) {
        unsafe { (self.overlay.set_overlay_transform_tracked_device_relative)(handle, device, transform) };
    }

    pub fn set_visible(&self, handle: u64, visible: bool) {
        unsafe {
            if visible {
                (self.overlay.show_overlay)(handle);
            } else {
                (self.overlay.hide_overlay)(handle);
            }
        }
    }

    /// RGBA, 4 bytes a pixel, rows top to bottom.
    pub fn set_raw(&self, handle: u64, rgba: &mut [u8], width: u32, height: u32) -> Result<(), String> {
        check(unsafe { (self.overlay.set_overlay_raw)(handle, rgba.as_mut_ptr().cast(), width, height, 4) })
    }
}

// The tables are plain function pointers into the OpenVR library, whose calls may come from any
// thread: the panel draws on one and tracks on another.
unsafe impl Send for OpenVr {}
unsafe impl Sync for OpenVr {}

impl Drop for OpenVr {
    fn drop(&mut self) {
        unsafe {
            if let Ok(shutdown) = self.lib.get::<VrShutdownInternalFn>(b"VR_ShutdownInternal\0") {
                shutdown();
            }
        }
    }
}

fn input_check(error: i32) -> Result<(), String> {
    if error == 0 { Ok(()) } else { Err(format!("OpenVR input error {error}")) }
}

fn check(error: i32) -> Result<(), String> {
    if error == 0 { Ok(()) } else { Err(format!("OpenVR overlay error {error}")) }
}

/// Safety: `T` must mirror the function table of `version`.
unsafe fn interface<T>(lib: &Library, version: &str) -> Result<&'static T, String> {
    let get: Symbol<VrGetGenericInterfaceFn> = lib.get(b"VR_GetGenericInterface\0").map_err(|e| e.to_string())?;
    // `FnTable:` asks for the flat C table rather than the C++ object.
    let name = CString::new(format!("FnTable:{version}")).unwrap();
    let mut error = 0i32;
    let ptr = get(name.as_ptr(), &mut error);
    if error != 0 || ptr == 0 {
        return Err(format!("{version} unavailable (error {error})"));
    }
    // The table lives as long as the library, which `OpenVr` keeps loaded.
    Ok(&*(ptr as *const T))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_match_the_header() {
        let ptr = std::mem::size_of::<*const c_void>();
        assert_eq!(std::mem::offset_of!(SystemFnTable, get_device_to_absolute_tracking_pose), 12 * ptr);
        assert_eq!(std::mem::offset_of!(SystemFnTable, get_tracked_device_index_for_controller_role), 18 * ptr);
        assert_eq!(std::mem::offset_of!(SystemFnTable, poll_next_event), 30 * ptr);
        assert_eq!(std::mem::offset_of!(SystemFnTable, acknowledge_quit_exiting), 47 * ptr);
        assert_eq!(std::mem::offset_of!(OverlayFnTable, create_overlay), ptr);
        assert_eq!(std::mem::offset_of!(OverlayFnTable, destroy_overlay), 3 * ptr);
        assert_eq!(std::mem::offset_of!(OverlayFnTable, set_overlay_color), 14 * ptr);
        assert_eq!(std::mem::offset_of!(OverlayFnTable, set_overlay_alpha), 16 * ptr);
        assert_eq!(std::mem::offset_of!(OverlayFnTable, set_overlay_sort_order), 20 * ptr);
        assert_eq!(std::mem::offset_of!(OverlayFnTable, set_overlay_width_in_meters), 22 * ptr);
        assert_eq!(std::mem::offset_of!(OverlayFnTable, set_overlay_transform_tracked_device_relative), 35 * ptr);
        assert_eq!(std::mem::offset_of!(OverlayFnTable, show_overlay), 43 * ptr);
        assert_eq!(std::mem::offset_of!(OverlayFnTable, set_overlay_raw), 62 * ptr);
        assert_eq!(std::mem::size_of::<TrackedDevicePose>(), 80);
        assert_eq!(std::mem::offset_of!(InputFnTable, get_pose_action_data_relative_to_now), 7 * ptr);
        assert_eq!(std::mem::size_of::<DigitalActionData>(), 24);
        assert_eq!(std::mem::size_of::<PoseActionData>(), 96);
        assert_eq!(std::mem::size_of::<ActiveActionSet>(), 32);
        assert_eq!(std::mem::size_of::<Event>(), 64);
    }
}
