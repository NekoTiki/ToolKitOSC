//! "Launch with SteamVR" — registers this app as a SteamVR auto-launch application through
//! OpenVR's manifest API (`IVRApplications::AddApplicationManifest`/`SetApplicationAutoLaunch`),
//! the same mechanism apps like Advanced Settings/XSOverlay/VRCFaceTracking use to appear under
//! SteamVR's own Settings > Startup/Shutdown > Manage Add-ons.
//!
//! There's no maintained, build-tooling-free Rust binding for this: the community `openvr`/
//! `openvr_sys` crates need CMake and bindgen/libclang to vendor-build the C++ SDK, which would add
//! a new native toolchain requirement to every dev/CI machine. Instead this loads `openvr_api.dll`
//! at runtime via `libloading` and calls its stable, flat C ABI directly (`openvr_capi.h`) - the
//! same technique other non-C++ OpenVR bindings (e.g. Python's) use. The `IVRApplicationsFnTable`
//! layout below is taken verbatim (field order matters, it's a plain vtable) from Valve's
//! `openvr_capi.h` for interface version `IVRApplications_008`:
//! https://github.com/ValveSoftware/openvr/blob/master/headers/openvr_capi.h
use std::ffi::{c_char, c_void, CString};
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};
use serde::Serialize;

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

/// Locates SteamVR's own copy of openvr_api.dll via the Steam client's install-path registry key
/// (the same key Steam itself relies on) rather than bundling a copy of the dll in this repo.
/// Checks the default Steam library first, then any additional libraries listed in
/// `libraryfolders.vdf` - a manually relocated SteamVR install outside of Steam's own library
/// system isn't found, same class of limitation as `commands::find_intiface_central`.
#[cfg(target_os = "windows")]
pub fn find_openvr_api_dll() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    const RELATIVE_DLL_PATH: &str = "steamapps/common/SteamVR/bin/win64/openvr_api.dll";

    let steam_path: String = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Valve\\Steam")
        .ok()?
        .get_value("SteamPath")
        .ok()?;

    let default_candidate = PathBuf::from(&steam_path).join(RELATIVE_DLL_PATH);
    if default_candidate.is_file() {
        return Some(default_candidate);
    }

    // SteamVR might live in a secondary library on another drive, listed in this VDF file as
    // `"path"		"D:\\SteamLibrary"` entries. A full VDF parser is overkill for this - the only
    // thing worth extracting is the quoted string following a `"path"` key on its own line.
    let contents = std::fs::read_to_string(PathBuf::from(&steam_path).join("steamapps/libraryfolders.vdf")).ok()?;

    contents.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("\"path\"")?.trim();
        let library = rest.trim_matches('"').replace("\\\\", "\\");
        let candidate = PathBuf::from(library).join(RELATIVE_DLL_PATH);
        candidate.is_file().then_some(candidate)
    })
}

#[cfg(not(target_os = "windows"))]
pub fn find_openvr_api_dll() -> Option<PathBuf> {
    // SteamVR's Linux/macOS install layouts aren't covered - Windows+SteamVR+VRChat is this
    // feature's only realistic combination.
    None
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
    /// Loads the dll and calls `VR_InitInternal`. Safety: `dll_path` must point at a genuine
    /// `openvr_api.dll` - this is only ever called with a path this module found itself.
    unsafe fn init(dll_path: &Path) -> Result<Self, String> {
        let lib = Library::new(dll_path).map_err(|e| format!("Failed to load openvr_api.dll: {e}"))?;

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

        let version = CString::new("IVRApplications_008").unwrap();
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

#[derive(Serialize)]
struct ManifestLocale {
    name: &'static str,
    description: &'static str
}

#[derive(Serialize)]
struct ManifestStrings {
    en_us: ManifestLocale
}

#[derive(Serialize)]
struct ManifestApp {
    app_key: &'static str,
    launch_type: &'static str,
    binary_path_windows: String,
    is_dashboard_overlay: bool,
    strings: ManifestStrings
}

#[derive(Serialize)]
struct Manifest {
    source: &'static str,
    applications: [ManifestApp; 1]
}

/// `binary_path_windows` is the current exe's full absolute path (rather than a name resolved
/// relative to the manifest file, which is how most shipped .vrmanifest examples do it) so this
/// doesn't depend on the manifest living next to the exe - it's written into this app's own data
/// directory instead, which is simpler to manage across updates/reinstalls.
fn write_manifest(manifest_path: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;

    let manifest = Manifest {
        source: "builtin",
        applications: [ManifestApp {
            app_key: APP_KEY,
            launch_type: "binary",
            binary_path_windows: exe.to_string_lossy().into_owned(),
            is_dashboard_overlay: false,
            strings: ManifestStrings {
                en_us: ManifestLocale {
                    name: "ToolKitOSC",
                    description: "ToolKitOSC"
                }
            }
        }]
    };

    let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    std::fs::write(manifest_path, json).map_err(|e| e.to_string())
}

/// Registers (or re-registers - this is idempotent, SteamVR just no-ops on an unchanged manifest)
/// this app with SteamVR and sets its auto-launch flag. Requires SteamVR to already be running -
/// there's no OpenVR application type whose Init doesn't need a live vrserver to talk to, so this
/// genuinely cannot succeed while SteamVR is closed (see describe_init_error above).
pub fn set_auto_launch(manifest_path: &Path, enable: bool) -> Result<(), String> {
    let dll_path = find_openvr_api_dll().ok_or("SteamVR was not found on this system")?;
    write_manifest(manifest_path)?;
    let manifest_c = path_to_cstring(manifest_path)?;

    unsafe {
        let vr = OpenVr::init(&dll_path)?;
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
    let dll_path = find_openvr_api_dll().ok_or("SteamVR was not found on this system")?;
    write_manifest(manifest_path)?;
    let manifest_c = path_to_cstring(manifest_path)?;

    unsafe {
        let vr = OpenVr::init(&dll_path)?;
        let apps = vr.applications()?;

        let error = (apps.add_application_manifest)(manifest_c.as_ptr().cast_mut(), false);
        if error != 0 {
            return Err(describe_application_error(error));
        }

        let mut key = CString::new(APP_KEY).unwrap().into_bytes_with_nul();
        Ok((apps.get_application_auto_launch)(key.as_mut_ptr().cast()))
    }
}
