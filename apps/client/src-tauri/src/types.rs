//! Wire types mirroring `@vrc-osc-toolkit/shared-ui/types/osc.ts`. Kept in sync by hand — if you
//! change one side, change the other.
use serde::{Deserialize, Serialize};

/// JS has a single `number` type; OSC/Rust distinguish int32 vs float32. We serialize every
/// numeric OSC arg as an `f64` (or, when it has no fractional part, still an `f64` that
/// `serde_json` prints without a decimal point) so the JSON on the wire matches what the
/// original Node implementation produced — see `osc::codec` for the rounding rule this mirrors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OscArg {
    Bool(bool),
    Number(f64),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OscMessage {
    pub address: String,
    pub args: Vec<OscArg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OscCommand {
    pub address: String,
    pub args: Vec<OscArg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarParameterAddress {
    pub address: String,
    #[serde(rename = "type")]
    pub kind: String, // 'Bool' | 'Float' | 'Int'
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarParameter {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<AvatarParameterAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<AvatarParameterAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarDetails {
    pub id: String,
    pub name: String,
    pub hash: i64,
    pub parameters: Vec<AvatarParameter>,
}
