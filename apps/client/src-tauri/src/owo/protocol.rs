//! The OWO app's protocol: plain text over UDP to port 54020, the way github.com/MrMineO5/owo-skin-rs
//! (MIT) speaks it. `STOP`, `GAMEUNAVAILABLE` and `OWO_Close` come from reverse-engineered notes
//! (github.com/VixenCreations/Unofficial-Python-OWO-SDK), not from OWO.
//!
//! - `ping` (broadcast, or to a typed address) gets `okay`; `<game>*AUTH*` then gets `pong`.
//! - Game ID `0` is anonymous: live sensations only, no baked ones, nothing to register.
//! - A sensation is `<game>*SENSATION*<freq>,<tenths>,<intensity>,<ramp up ms>,<ramp down ms>,<exit
//!   tenths>,<name>|<muscle>%<percent>,…`. A new one replaces the one playing.
use super::model::Muscle;
use super::touch::Levels;

pub const PORT: u16 = 54020;
/// Anonymous. Registering a game with OWO would only change this.
pub const GAME_ID: &str = "0";
const NAME: &str = "TKOSC";

/// One micro sensation, before it's put on muscles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Micro {
    /// Hz, 1 to 100.
    pub frequency: u8,
    /// Tenths of a second, 1 to 200.
    pub tenths: u8,
    /// 0 to 100.
    pub intensity: u8,
    pub ramp_up_ms: u16,
    pub ramp_down_ms: u16,
    pub exit_tenths: u8
}

impl Micro {
    pub fn new(frequency: u8, tenths: u8, intensity: u8) -> Self {
        Self { frequency, tenths, intensity, ramp_up_ms: 0, ramp_down_ms: 0, exit_tenths: 0 }
    }
}

/// What we send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Packet {
    Ping,
    Auth,
    Sensation { micro: Micro, muscles: Vec<(Muscle, u8)> },
    /// Stops whatever plays.
    Stop,
    /// We're leaving.
    GameUnavailable
}

impl Packet {
    pub fn text(&self) -> String {
        match self {
            Packet::Ping => "ping".into(),
            Packet::Auth => format!("{GAME_ID}*AUTH*"),
            Packet::Sensation { micro: m, muscles } => {
                let muscles: Vec<String> = muscles.iter().map(|(muscle, percent)| format!("{}%{}", muscle.index(), (*percent).min(100))).collect();
                format!(
                    "{GAME_ID}*SENSATION*{},{},{},{},{},{},{NAME}|{}",
                    m.frequency.clamp(1, 100),
                    m.tenths.clamp(1, 200),
                    m.intensity.min(100),
                    m.ramp_up_ms.min(2000),
                    m.ramp_down_ms.min(2000),
                    m.exit_tenths.min(200),
                    muscles.join(",")
                )
            }
            Packet::Stop => format!("{GAME_ID}*STOP"),
            Packet::GameUnavailable => format!("{GAME_ID}*GAMEUNAVAILABLE")
        }
    }

    /// Reads one of ours back, for the tester.
    #[cfg(debug_assertions)]
    pub fn parse(text: &str) -> Option<Packet> {
        if text == "ping" {
            return Some(Packet::Ping);
        }
        let (_game, rest) = text.split_once('*')?;
        let (kind, body) = rest.split_once('*').unwrap_or((rest, ""));
        match kind {
            "AUTH" => Some(Packet::Auth),
            "STOP" => Some(Packet::Stop),
            "GAMEUNAVAILABLE" => Some(Packet::GameUnavailable),
            "SENSATION" => {
                let (sensation, muscles) = body.split_once('|')?;
                let fields: Vec<&str> = sensation.split(',').collect();
                let [frequency, tenths, intensity, ramp_up_ms, ramp_down_ms, exit_tenths, _name] = fields[..] else {
                    return None;
                };
                let micro = Micro {
                    frequency: frequency.parse().ok()?,
                    tenths: tenths.parse().ok()?,
                    intensity: intensity.parse().ok()?,
                    ramp_up_ms: ramp_up_ms.parse().ok()?,
                    ramp_down_ms: ramp_down_ms.parse().ok()?,
                    exit_tenths: exit_tenths.parse().ok()?
                };
                let muscles = muscles
                    .split(',')
                    .filter(|part| !part.is_empty())
                    .map(|part| {
                        let (index, percent) = part.split_once('%')?;
                        Some((Muscle::from_index(index.parse().ok()?)?, percent.parse().ok()?))
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some(Packet::Sensation { micro, muscles })
            }
            _ => None
        }
    }
}

/// One sensation playing `levels` (0–1 per muscle): the strongest sets the intensity, and each
/// muscle gets its share of it. None when nothing plays.
pub fn sensation(frequency: u8, tenths: u8, levels: &Levels) -> Option<Packet> {
    let strongest = levels.iter().map(|(_, level)| level.clamp(0.0, 1.0)).fold(0.0, f64::max);
    let intensity = (strongest * 100.0).round() as u8;
    if intensity == 0 {
        return None;
    }
    let muscles = levels
        .iter()
        .map(|(muscle, level)| (*muscle, (level.clamp(0.0, 1.0) / strongest * 100.0).round() as u8))
        .filter(|(_, percent)| *percent > 0)
        .collect();
    Some(Packet::Sensation { micro: Micro::new(frequency, tenths, intensity), muscles })
}

/// What the OWO app says back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply {
    /// Heard our ping: identify.
    Okay,
    /// Connected.
    Pong,
    /// It closed the connection.
    Close
}

pub fn reply(text: &str) -> Option<Reply> {
    match text.trim() {
        "okay" => Some(Reply::Okay),
        "pong" => Some(Reply::Pong),
        "OWO_Close" => Some(Reply::Close),
        _ => None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_what_owo_skin_rs_writes() {
        // owo-skin-rs: Sensation::micro_sensation(100, 0.3, 60, 0.0, 0.0, 0.0, "TKOSC") on muscles 8 and 9.
        let packet = Packet::Sensation { micro: Micro::new(100, 3, 60), muscles: vec![(Muscle::LumbarR, 100), (Muscle::LumbarL, 100)] };
        assert_eq!(packet.text(), "0*SENSATION*100,3,60,0,0,0,TKOSC|8%100,9%100");
        let micro = Micro { frequency: 50, tenths: 10, intensity: 80, ramp_up_ms: 200, ramp_down_ms: 500, exit_tenths: 2 };
        assert_eq!(Packet::Sensation { micro, muscles: vec![(Muscle::PectoralR, 40)] }.text(), "0*SENSATION*50,10,80,200,500,2,TKOSC|0%40");
        assert_eq!(Packet::Auth.text(), "0*AUTH*");
        assert_eq!(Packet::Stop.text(), "0*STOP");
        assert_eq!(Packet::GameUnavailable.text(), "0*GAMEUNAVAILABLE");
    }

    #[test]
    fn keeps_values_in_owos_ranges() {
        let micro = Micro { frequency: 0, tenths: 255, intensity: 150, ramp_up_ms: 9000, ramp_down_ms: 0, exit_tenths: 0 };
        assert_eq!(Packet::Sensation { micro, muscles: vec![(Muscle::ArmL, 200)] }.text(), "0*SENSATION*1,200,100,2000,0,0,TKOSC|5%100");
    }

    #[test]
    fn reads_back_what_it_writes() {
        let micro = Micro { frequency: 70, tenths: 2, intensity: 45, ramp_up_ms: 100, ramp_down_ms: 0, exit_tenths: 0 };
        for packet in [
            Packet::Ping,
            Packet::Auth,
            Packet::Stop,
            Packet::GameUnavailable,
            Packet::Sensation { micro, muscles: vec![(Muscle::PectoralL, 100), (Muscle::LumbarL, 75)] }
        ] {
            assert_eq!(Packet::parse(&packet.text()), Some(packet));
        }
        assert_eq!(Packet::parse("0*SENSATION*100,3|0%100"), None);
        assert_eq!(Packet::parse("0*SENSATION*100,3,60,0,0,0,TKOSC|12%100"), None, "only ten muscles");
    }

    #[test]
    fn levels_become_one_sensation() {
        let packet = sensation(100, 3, &Levels::from([(Muscle::PectoralL, 0.6), (Muscle::LumbarL, 0.3), (Muscle::ArmR, 0.0)]));
        assert_eq!(packet.unwrap().text(), "0*SENSATION*100,3,60,0,0,0,TKOSC|1%100,9%50");
        assert_eq!(sensation(100, 3, &Levels::from([(Muscle::PectoralL, 0.0)])), None);
        assert_eq!(sensation(100, 3, &Levels::new()), None);
    }

    #[test]
    fn reads_the_apps_replies() {
        assert_eq!(reply("okay"), Some(Reply::Okay));
        assert_eq!(reply("pong\n"), Some(Reply::Pong));
        assert_eq!(reply("OWO_Close"), Some(Reply::Close));
        assert_eq!(reply("hello"), None);
    }
}
