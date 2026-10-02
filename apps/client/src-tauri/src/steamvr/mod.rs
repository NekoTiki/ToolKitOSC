//! "Launch with SteamVR" — registers this app as a SteamVR auto-launch application through
//! OpenVR's manifest API (`IVRApplications::AddApplicationManifest`/`SetApplicationAutoLaunch`),
//! the same mechanism apps like Advanced Settings/XSOverlay/VRCFaceTracking use to appear under
//! SteamVR's own Settings > Startup/Shutdown > Manage Add-ons. Works with SteamVR on Windows and
//! its native Linux build; the per-platform bits (where the OpenVR library lives, which manifest
//! key holds our binary) are in `windows.rs`/`linux.rs`.
//!
//! There's no maintained, build-tooling-free Rust binding for this: the community `openvr`/
//! `openvr_sys` crates need CMake and bindgen/libclang to vendor-build the C++ SDK, which would add
//! a new native toolchain requirement to every dev/CI machine. Instead this loads SteamVR's own
//! `openvr_api.dll`/`libopenvr_api.so` at runtime via `libloading` and calls its stable, flat C ABI
//! directly (`openvr_capi.h`) - the same technique other non-C++ OpenVR bindings (e.g. Python's)
//! use. The `IVRApplicationsFnTable` layout below is taken verbatim (field order matters, it's a
//! plain table of function pointers) from Valve's `openvr_capi.h` for interface version
//! `IVRApplications_008`, and is identical on both platforms:
//! https://github.com/ValveSoftware/openvr/blob/master/headers/openvr_capi.h
use std::ffi::{c_char, c_void, CString};
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::{app_binary, openvr_paths_file, BINARY_PATH_KEY, OPENVR_API_PATH, SUPPORTED};
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows::{app_binary, openvr_paths_file, BINARY_PATH_KEY, OPENVR_API_PATH, SUPPORTED};
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod unsupported;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
use unsupported::{app_binary, openvr_paths_file, BINARY_PATH_KEY, OPENVR_API_PATH, SUPPORTED};

/// Reverse-DNS key SteamVR uses to identify this app in its manifest registry - matches the Tauri
/// app identifier (tauri.conf.json) and must stay stable across versions/reinstalls, since it's
/// how SetApplicationAutoLaunch/GetApplicationAutoLaunch address an already-registered app.
const APP_KEY: &str = "io.github.nekotiki.toolkitosc";

const APP_TYPE_BACKGROUND: i32 = 3; // EVRApplicationType_VRApplication_Background
const INIT_ERROR_NO_SERVER_FOR_BACKGROUND_APP: i32 = 121;
const APPLICATION_ERROR_UNKNOWN_APPLICATION: i32 = 104;

// Only the vtable members this module actually calls are given real signatures. Everything
// between them is declared as opaque, pointer-sized filler purely to keep the following members'
// offsets correct - `#[repr(C)]` lays fields out in declaration order, and every member of this
// table (function pointers) is the same size, so a same-sized placeholder preserves layout
// without needing every unused method's exact signature.
#[repr(C)]
struct IVRApplicationsFnTable {
    add_application_manifest: unsafe extern "system" fn(*mut c_char, bool) -> i32,
    remove_application_manifest: unsafe extern "system" fn(*mut c_char) -> i32,
    is_application_installed: unsafe extern "system" fn(*mut c_char) -> bool,
    // GetApplicationCount .. GetApplicationPropertyUint64
    _unused: [*const c_void; 14],
    set_application_auto_launch: unsafe extern "system" fn(*mut c_char, bool) -> i32,
    get_application_auto_launch: unsafe extern "system" fn(*mut c_char) -> bool
}

type VrInitInternalFn = unsafe extern "system" fn(*mut i32, i32) -> isize;
type VrShutdownInternalFn = unsafe extern "system" fn();
type VrGetGenericInterfaceFn = unsafe extern "system" fn(*const c_char, *mut i32) -> isize;

/// Whether SteamVR can exist on this platform at all, as opposed to just not being installed.
pub fn is_supported() -> bool {
    SUPPORTED
}

/// Locates SteamVR's own copy of the OpenVR library rather than bundling one in this repo. Checks
/// the runtime SteamVR registered in `openvrpaths.vrpath` first (the same file the OpenVR library
/// itself reads to find the runtime), then a `SteamVR` folder in every Steam library - a manually
/// relocated SteamVR install that's in neither isn't found, same class of limitation as
/// `commands::find_intiface_central`.
pub fn find_openvr_api() -> Option<PathBuf> {
    registered_runtimes()
        .into_iter()
        .chain(
            crate::steam::libraries()
                .into_iter()
                .map(|library| library.join("steamapps/common/SteamVR")),
        )
        .map(|runtime| runtime.join(OPENVR_API_PATH))
        .find(|path| path.is_file())
}

/// The `runtime` entries of `openvrpaths.vrpath`, a JSON file SteamVR writes whenever it starts.
fn registered_runtimes() -> Vec<PathBuf> {
    let Some(contents) = openvr_paths_file().and_then(|path| std::fs::read_to_string(path).ok()) else {
        return Vec::new();
    };

    serde_json::from_str::<serde_json::Value>(&contents)
        .ok()
        .and_then(|paths| {
            let runtimes = paths.get("runtime")?.as_array()?;
            Some(runtimes.iter().filter_map(|runtime| runtime.as_str().map(PathBuf::from)).collect())
        })
        .unwrap_or_default()
}

fn describe_init_error(code: i32) -> String {
    if code == INIT_ERROR_NO_SERVER_FOR_BACKGROUND_APP {
        "SteamVR isn't running. Start SteamVR, then try again.".to_string()
    } else {
        format!("Failed to initialize OpenVR (error {code})")
    }
}

fn describe_application_error(code: i32) -> String {
    if code == APPLICATION_ERROR_UNKNOWN_APPLICATION {
        "SteamVR needs to be restarted once after first registering this app - restart SteamVR \
         and try again."
            .to_string()
    } else {
        format!("SteamVR rejected the request (error {code})")
    }
}

fn path_to_cstring(path: &Path) -> Result<CString, String> {
    let str = path
        .to_str()
        .ok_or_else(|| "Path contains invalid characters".to_string())?;
    CString::new(str).map_err(|e| e.to_string())
}

struct OpenVr {
    lib: Library,
    initialized: bool
}

impl OpenVr {
    /// Loads the library and calls `VR_InitInternal`. Safety: `lib_path` must point at a genuine
    /// OpenVR library - this is only ever called with a path this module found itself.
    unsafe fn init(lib_path: &Path) -> Result<Self, String> {
        let lib = Library::new(lib_path).map_err(|e| format!("Failed to load the OpenVR library: {e}"))?;

        let mut this = Self { lib, initialized: false };

        let init: Symbol<VrInitInternalFn> = this
            .lib
            .get(b"VR_InitInternal\0")
            .map_err(|e| e.to_string())?;

        let mut error = 0i32;
        init(&mut error, APP_TYPE_BACKGROUND);
        drop(init);

        if error != 0 {
            return Err(describe_init_error(error));
        }

        this.initialized = true;
        Ok(this)
    }

    unsafe fn applications(&self) -> Result<&IVRApplicationsFnTable, String> {
        let get_interface: Symbol<VrGetGenericInterfaceFn> = self
            .lib
            .get(b"VR_GetGenericInterface\0")
            .map_err(|e| e.to_string())?;

        // The `FnTable:` prefix asks for the flat C function table `IVRApplicationsFnTable` mirrors.
        // Without it OpenVR hands back the C++ interface object instead, whose first field is a
        // vtable pointer, not the first function.
        let version = CString::new("FnTable:IVRApplications_008").unwrap();
        let mut error = 0i32;
        let ptr = get_interface(version.as_ptr(), &mut error);

        if error != 0 || ptr == 0 {
            return Err(format!("IVRApplications interface unavailable (error {error})"));
        }

        Ok(&*(ptr as *const IVRApplicationsFnTable))
    }
}

impl Drop for OpenVr {
    fn drop(&mut self) {
        if !self.initialized {
            return;
        }

        unsafe {
            if let Ok(shutdown) = self.lib.get::<VrShutdownInternalFn>(b"VR_ShutdownInternal\0") {
                shutdown();
            }
        }
    }
}

/// The binary path is our app's full absolute path (rather than a name resolved relative to the
/// manifest file, which is how most shipped .vrmanifest examples do it) so this doesn't depend on
/// the manifest living next to the binary - it's written into this app's own data directory
/// instead, which is simpler to manage across updates/reinstalls. Its key differs per platform
/// (`binary_path_windows`/`binary_path_linux`), hence building the JSON as a value.
fn write_manifest(manifest_path: &Path) -> Result<(), String> {
    let binary = app_binary()?;

    let mut app = serde_json::json!({
        "app_key": APP_KEY,
        "launch_type": "binary",
        "is_dashboard_overlay": false,
        "strings": {
            "en_us": {
                "name": "ToolKitOSC",
                "description": "ToolKitOSC"
            }
        }
    });
    app[BINARY_PATH_KEY] = binary.to_string_lossy().into_owned().into();

    let manifest = serde_json::json!({
        "source": "builtin",
        "applications": [app]
    });

    let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    std::fs::write(manifest_path, json).map_err(|e| e.to_string())
}

/// Registers (or re-registers - this is idempotent, SteamVR just no-ops on an unchanged manifest)
/// this app with SteamVR and sets its auto-launch flag. Requires SteamVR to already be running -
/// there's no OpenVR application type whose Init doesn't need a live vrserver to talk to, so this
/// genuinely cannot succeed while SteamVR is closed (see describe_init_error above).
pub fn set_auto_launch(manifest_path: &Path, enable: bool) -> Result<(), String> {
    let lib_path = find_openvr_api().ok_or("SteamVR was not found on this system")?;
    write_manifest(manifest_path)?;
    let manifest_c = path_to_cstring(manifest_path)?;

    unsafe {
        let vr = OpenVr::init(&lib_path)?;
        let apps = vr.applications()?;

        let error = (apps.add_application_manifest)(manifest_c.as_ptr().cast_mut(), false);
        if error != 0 {
            return Err(describe_application_error(error));
        }

        let mut key = CString::new(APP_KEY).unwrap().into_bytes_with_nul();
        let error = (apps.set_application_auto_launch)(key.as_mut_ptr().cast(), enable);
        if error != 0 {
            return Err(describe_application_error(error));
        }
    }

    Ok(())
}

/// Best-effort read of SteamVR's own auto-launch flag for this app - used to reconcile the
/// Settings toggle with reality (e.g. if the user removed it via SteamVR's own Manage Add-ons UI)
/// rather than only trusting whatever this app last set it to. Registers the manifest first (same
/// as `set_auto_launch`) since a freshly-restarted SteamVR won't know about this app otherwise.
pub fn get_auto_launch(manifest_path: &Path) -> Result<bool, String> {
    let lib_path = find_openvr_api().ok_or("SteamVR was not found on this system")?;
    write_manifest(manifest_path)?;
    let manifest_c = path_to_cstring(manifest_path)?;

    unsafe {
        let vr = OpenVr::init(&lib_path)?;
        let apps = vr.applications()?;

        let error = (apps.add_application_manifest)(manifest_c.as_ptr().cast_mut(), false);
        if error != 0 {
            return Err(describe_application_error(error));
        }

        let mut key = CString::new(APP_KEY).unwrap().into_bytes_with_nul();
        Ok((apps.get_application_auto_launch)(key.as_mut_ptr().cast()))
    }
}
