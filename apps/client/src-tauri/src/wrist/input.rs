//! The panel's SteamVR Input actions: `click` (the trigger by default) and `pointer` (each
//! controller's tip pose, which SteamVR places where that controller model points from). Written
//! to the app's data folder at start, so SteamVR's controller settings can rebind them.
use std::path::PathBuf;

const MANIFEST: &str = r#"{
  "default_bindings": [
    { "controller_type": "knuckles", "binding_url": "bindings_knuckles.json" },
    { "controller_type": "oculus_touch", "binding_url": "bindings_oculus_touch.json" },
    { "controller_type": "vive_controller", "binding_url": "bindings_vive_controller.json" }
  ],
  "actions": [
    { "name": "/actions/wrist/in/click", "type": "boolean" },
    { "name": "/actions/wrist/in/pointer", "type": "pose" }
  ],
  "action_sets": [{ "name": "/actions/wrist", "usage": "leftright" }],
  "localization": [{
    "language_tag": "en_US",
    "/actions/wrist": "ToolKitOSC wrist panel",
    "/actions/wrist/in/click": "Press on the panel",
    "/actions/wrist/in/pointer": "Point at the panel"
  }]
}"#;

fn bindings(controller_type: &str) -> String {
    let hand = |side: &str| {
        format!(r#"{{ "path": "/user/hand/{side}/input/trigger", "mode": "trigger", "inputs": {{ "click": {{ "output": "/actions/wrist/in/click" }} }} }}"#)
    };
    let pose = |side: &str| format!(r#"{{ "output": "/actions/wrist/in/pointer", "path": "/user/hand/{side}/pose/tip" }}"#);
    format!(
        r#"{{ "controller_type": "{controller_type}", "bindings": {{ "/actions/wrist": {{ "sources": [{}, {}], "poses": [{}, {}] }} }} }}"#,
        hand("left"),
        hand("right"),
        pose("left"),
        pose("right")
    )
}

/// Writes the manifest and its default bindings, and returns the manifest's path.
pub fn write_manifest() -> Result<PathBuf, String> {
    let dir = directories::BaseDirs::new().ok_or("no home folder")?.config_dir().join("io.github.nekotiki.toolkitosc").join("steamvr-input");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for controller_type in ["knuckles", "oculus_touch", "vive_controller"] {
        std::fs::write(dir.join(format!("bindings_{controller_type}.json")), bindings(controller_type)).map_err(|e| e.to_string())?;
    }
    let path = dir.join("actions.json");
    std::fs::write(&path, MANIFEST).map_err(|e| e.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_files_are_json() {
        serde_json::from_str::<serde_json::Value>(MANIFEST).unwrap();
        let bindings: serde_json::Value = serde_json::from_str(&bindings("knuckles")).unwrap();
        assert_eq!(bindings.pointer("/bindings/~1actions~1wrist/poses/1/path").unwrap(), "/user/hand/right/pose/tip");
    }
}
