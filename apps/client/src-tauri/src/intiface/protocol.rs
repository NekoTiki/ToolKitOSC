//! The Buttplug v3 messages ToolKitOSC uses, as plain JSON: enough to list ScalarCmd actuators,
//! read batteries and send ScalarCmd. Spec: https://buttplug-spec.docs.buttplug.io
//!
//! ScalarCmd covers vibrate, rotate, oscillate and the rest; only `ActuatorType` differs. An
//! actuator's index is its position in the device's ScalarCmd list, and a battery sensor's is its
//! position in SensorReadCmd. Toys that only offer RotateCmd or LinearCmd aren't covered yet.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Actuator {
    pub index: u32,
    pub description: String,
    pub actuator_type: String
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub index: u32,
    pub name: String,
    pub actuators: Vec<Actuator>,
    /// Position of its Battery entry in SensorReadCmd, if it has one. Most BLE toys don't.
    pub battery_sensor_index: Option<u32>,
    /// 0–1, once a reading came back.
    pub battery: Option<f64>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Attributes {
    feature_descriptor: Option<String>,
    actuator_type: Option<String>,
    sensor_type: Option<String>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeviceInfo {
    device_name: String,
    device_index: u32,
    #[serde(default)]
    device_messages: DeviceMessages
}

#[derive(Debug, Default, Deserialize)]
struct DeviceMessages {
    #[serde(rename = "ScalarCmd", default)]
    scalar_cmd: Vec<Attributes>,
    #[serde(rename = "SensorReadCmd", default)]
    sensor_read_cmd: Vec<Attributes>
}

impl DeviceInfo {
    pub fn to_device(&self) -> Device {
        let battery_sensor_index = self
            .device_messages
            .sensor_read_cmd
            .iter()
            .position(|sensor| sensor.sensor_type.as_deref() == Some("Battery"))
            .map(|index| index as u32);

        Device {
            index: self.device_index,
            name: self.device_name.clone(),
            actuators: self
                .device_messages
                .scalar_cmd
                .iter()
                .enumerate()
                .map(|(index, actuator)| {
                    let actuator_type = actuator.actuator_type.clone().unwrap_or_else(|| "Vibrate".into());
                    Actuator {
                        index: index as u32,
                        description: actuator.feature_descriptor.clone().filter(|d| !d.is_empty()).unwrap_or_else(|| actuator_type.clone()),
                        actuator_type
                    }
                })
                .collect(),
            battery_sensor_index,
            battery: None
        }
    }
}

/// One message from the server.
#[derive(Debug)]
pub enum Incoming {
    ServerInfo,
    DeviceList(Vec<DeviceInfo>),
    DeviceAdded(DeviceInfo),
    DeviceRemoved(u32),
    Battery { device: u32, level: f64 },
    Error(String),
    Other
}

/// The server always sends an array of single-key objects.
pub fn decode(text: &str) -> Vec<Incoming> {
    let Ok(Value::Array(messages)) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };

    messages.into_iter().map(decode_one).collect()
}

fn decode_one(message: Value) -> Incoming {
    let Value::Object(map) = message else {
        return Incoming::Other;
    };
    let Some((kind, body)) = map.into_iter().next() else {
        return Incoming::Other;
    };

    match kind.as_str() {
        "ServerInfo" => Incoming::ServerInfo,
        "DeviceList" => Incoming::DeviceList(
            body.get("Devices").cloned().and_then(|devices| serde_json::from_value(devices).ok()).unwrap_or_default()
        ),
        "DeviceAdded" => match serde_json::from_value(body) {
            Ok(info) => Incoming::DeviceAdded(info),
            Err(_) => Incoming::Other
        },
        "DeviceRemoved" => match body.get("DeviceIndex").and_then(Value::as_u64) {
            Some(index) => Incoming::DeviceRemoved(index as u32),
            None => Incoming::Other
        },
        "SensorReading" => {
            let device = body.get("DeviceIndex").and_then(Value::as_u64);
            let battery = body.get("SensorType").and_then(Value::as_str) == Some("Battery");
            let level = body.get("Data").and_then(|data| data.get(0)).and_then(Value::as_f64);
            match (device, battery, level) {
                (Some(device), true, Some(level)) => Incoming::Battery { device: device as u32, level },
                _ => Incoming::Other
            }
        }
        "Error" => Incoming::Error(body.get("ErrorMessage").and_then(Value::as_str).unwrap_or_default().to_owned()),
        _ => Incoming::Other
    }
}

/// `{ "Kind": { ...body, "Id": id } }`. Ids are filled in when sending (see `with_id`).
pub fn message(kind: &str, body: Value) -> Value {
    json!({ kind: body })
}

/// Sets the message's `Id`, whatever its kind.
pub fn with_id(mut message: Value, id: u32) -> Value {
    if let Some(body) = message.as_object_mut().and_then(|map| map.values_mut().next()).and_then(Value::as_object_mut) {
        body.insert("Id".into(), json!(id));
    }
    message
}

pub fn request_server_info() -> Value {
    message("RequestServerInfo", json!({ "ClientName": "ToolKitOSC", "MessageVersion": 3 }))
}

pub fn battery_read(device: &Device) -> Option<Value> {
    let sensor = device.battery_sensor_index?;
    Some(message(
        "SensorReadCmd",
        json!({ "DeviceIndex": device.index, "SensorIndex": sensor, "SensorType": "Battery" })
    ))
}

/// One ScalarCmd for one device: `(actuator index, level, actuator type)` per actuator.
pub fn scalar_cmd(device: u32, scalars: &[(u32, f64, String)]) -> Value {
    let scalars: Vec<_> = scalars
        .iter()
        .map(|(index, level, kind)| json!({ "Index": index, "Scalar": level, "ActuatorType": kind }))
        .collect();
    message("ScalarCmd", json!({ "DeviceIndex": device, "Scalars": scalars }))
}

/// Host and port a `ws://` URL points at. IPv6 brackets are dropped.
pub fn target(url: &str) -> Option<(String, u16)> {
    let (secure, rest) = if let Some(rest) = url.strip_prefix("wss://") {
        (true, rest)
    } else {
        (false, url.strip_prefix("ws://")?)
    };
    let authority = rest.split(['/', '?', '#']).next()?;
    let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
        let (host, after) = bracketed.split_once(']')?;
        (host.to_owned(), after.strip_prefix(':').map(str::to_owned))
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) => (host.to_owned(), Some(port.to_owned())),
            None => (authority.to_owned(), None)
        }
    };
    let port = match port {
        Some(port) => port.parse().ok()?,
        None if secure => 443,
        None => 80
    };
    (!host.is_empty()).then_some((host.to_lowercase(), port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devices_keep_actuator_positions() {
        let incoming = decode(
            r#"[{"DeviceAdded":{"Id":0,"DeviceName":"Edge 2","DeviceIndex":3,"DeviceMessages":{
                "ScalarCmd":[{"StepCount":20,"FeatureDescriptor":"","ActuatorType":"Vibrate"},{"StepCount":20,"ActuatorType":"Rotate","FeatureDescriptor":"Head"}],
                "SensorReadCmd":[{"SensorType":"RSSI"},{"SensorType":"Battery"}]}}}]"#
        );
        let Incoming::DeviceAdded(info) = &incoming[0] else { panic!("{incoming:?}") };
        let device = info.to_device();
        assert_eq!(device.index, 3);
        assert_eq!(device.actuators[0].description, "Vibrate");
        assert_eq!(device.actuators[1].index, 1);
        assert_eq!(device.actuators[1].description, "Head");
        assert_eq!(device.battery_sensor_index, Some(1));
    }

    #[test]
    fn battery_and_errors_decode() {
        let incoming =
            decode(r#"[{"SensorReading":{"Id":4,"DeviceIndex":1,"SensorIndex":0,"SensorType":"Battery","Data":[0.8]}},{"Error":{"Id":1,"ErrorMessage":"nope","ErrorCode":1}}]"#);
        assert!(matches!(incoming[0], Incoming::Battery { device: 1, level } if level == 0.8));
        assert!(matches!(&incoming[1], Incoming::Error(message) if message == "nope"));
    }

    #[test]
    fn ids_are_filled_in() {
        let value = with_id(scalar_cmd(2, &[(0, 0.5, "Vibrate".into())]), 7);
        assert_eq!(value["ScalarCmd"]["Id"], 7);
        assert_eq!(value["ScalarCmd"]["Scalars"][0]["Scalar"], 0.5);
    }

    #[test]
    fn target_parses_hosts_and_ports() {
        assert_eq!(target("ws://localhost:12345"), Some(("localhost".into(), 12345)));
        assert_eq!(target("ws://[::1]:12345/"), Some(("::1".into(), 12345)));
        assert_eq!(target("wss://toys.example"), Some(("toys.example".into(), 443)));
        assert_eq!(target("http://nope"), None);
    }
}
