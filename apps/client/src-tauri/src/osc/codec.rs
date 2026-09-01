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

pub fn encode_arg(arg: &OscArg) -> rosc::OscType {
    match arg {
        OscArg::Bool(b) => rosc::OscType::Bool(*b),
        OscArg::Str(s) => rosc::OscType::String(s.clone()),
        OscArg::Number(n) => {
            if n.fract() == 0.0 {
                rosc::OscType::Int(*n as i32)
            } else {
                rosc::OscType::Float(*n as f32)
            }
        }
    }
}

pub fn encode_command(cmd: &crate::types::OscCommand) -> rosc::OscMessage {
    rosc::OscMessage {
        addr: cmd.address.clone(),
        args: cmd.args.iter().map(encode_arg).collect()
    }
}
