//! Conversions between `rosc`'s OSC wire types and our JSON-friendly [`OscArg`].
//!
//! Mirrors two behaviors from the original Node implementation (`osc` npm package):
//! - Decoding: normalizes numeric args exactly like the old `oscReceiver.on('message', ...)`
//!   handler did — integers pass through untouched, non-integer floats get rounded to 2 decimals.
//! - Encoding: infers the outgoing OSC type tag from whether the JS-side number is a whole
//!   number (int vs float), same as the `osc` package's `writeArguments` did for plain values.
use crate::types::OscArg;

pub fn decode_arg(arg: &rosc::OscType) -> Option<OscArg> {
    match arg {
        rosc::OscType::Bool(b) => Some(OscArg::Bool(*b)),
        rosc::OscType::String(s) => Some(OscArg::Str(s.clone())),
        rosc::OscType::Int(i) => Some(OscArg::Number(*i as f64)),
        rosc::OscType::Long(l) => Some(OscArg::Number(*l as f64)),
        rosc::OscType::Float(f) => Some(OscArg::Number(normalize_float(*f as f64))),
        rosc::OscType::Double(d) => Some(OscArg::Number(normalize_float(*d))),
        // Blob/Time/Char/Color/Midi/Array/Nil/Inf: not used by VRChat's avatar parameter
        // protocol, dropped rather than guessed at.
        _ => None,
    }
}

fn normalize_float(value: f64) -> f64 {
    if value.fract() == 0.0 {
        value
    } else {
        (value * 100.0).round() / 100.0
    }
}

pub fn decode_message(msg: &rosc::OscMessage) -> crate::types::OscMessage {
    crate::types::OscMessage {
        address: msg.addr.clone(),
        args: msg.args.iter().filter_map(decode_arg).collect()
    }
}

/// `expected_type` is the avatar parameter's declared wire type ("Bool"/"Float"/"Int"), when
/// known — see `encode_command`. It overrides the whole-number heuristic below, which otherwise
/// misfires for Float parameters whenever the value happens to land on a whole number (e.g. a
/// slider at exactly 0% or 100%): sending those as `Int` instead of `Float` desyncs VRChat's
/// avatar parameter state, since the two wire types aren't interchangeable for a Float parameter.
pub fn encode_arg(arg: &OscArg, expected_type: Option<&str>) -> rosc::OscType {
    match arg {
        OscArg::Bool(b) => rosc::OscType::Bool(*b),
        OscArg::Str(s) => rosc::OscType::String(s.clone()),
        OscArg::Number(n) => match expected_type {
            Some("Int") => rosc::OscType::Int(*n as i32),
            Some("Float") => rosc::OscType::Float(*n as f32),
            _ => {
                if n.fract() == 0.0 {
                    rosc::OscType::Int(*n as i32)
                } else {
                    rosc::OscType::Float(*n as f32)
                }
            }
        }
    }
}

pub fn encode_command(
    cmd: &crate::types::OscCommand,
    avatar_details: Option<&crate::types::AvatarDetails>
) -> rosc::OscMessage {
    // Prefer the current avatar's declared parameter type over guessing from the value's shape -
    // it's ground truth when we have it. Addresses that aren't avatar input parameters (OpenShock,
    // chatbox, etc.) fall through to the heuristic in `encode_arg`.
    let expected_type = avatar_details.and_then(|details| {
        details.parameters.iter().find_map(|p| {
            p.input
                .as_ref()
                .filter(|addr| addr.address == cmd.address)
                .map(|addr| addr.kind.as_str())
        })
    });

    rosc::OscMessage {
        addr: cmd.address.clone(),
        args: cmd.args.iter().map(|a| encode_arg(a, expected_type)).collect()
    }
}
