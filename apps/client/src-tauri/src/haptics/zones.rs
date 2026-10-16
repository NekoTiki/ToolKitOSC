//! The contacts SPS (VRCFury), OGB prefabs, VRCFury Haptics and legacy TPS put on an avatar, grouped
//! by zone, and what each zone reads for every kind of contact.
use std::collections::HashMap;

use serde::Serialize;

use super::depth::{self, PlugLength};

/// Where a zone's parameters come from. A VRCFury Haptics zone and an OGB zone with the same name
/// are two zones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    Ogb,
    Vfh,
    Tps
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZoneKind {
    /// `Orf`: an orifice.
    Socket,
    /// `Pen`: a penetrator.
    Plug,
    Touch
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ZoneId {
    pub family: Family,
    pub kind: ZoneKind,
    pub name: String
}

/// Splits an avatar parameter name (without `/avatar/parameters/`) into its zone and key, or
/// `None` when it isn't a contact parameter.
pub fn parse(name: &str) -> Option<(ZoneId, String)> {
    let mut parts = name.split('/');
    let (family, kind, zone, key) = match parts.next()? {
        "OGB" => {
            let kind = parts.next()?;
            let zone = parts.next()?;
            // OGB keys may have more parts.
            (Family::Ogb, kind, zone, parts.collect::<Vec<_>>().join("/"))
        }
        "TPS_Internal" => {
            let kind = parts.next()?;
            let zone = parts.next()?;
            (Family::Tps, kind, zone, parts.collect::<Vec<_>>().join("/"))
        }
        "VFH" if parts.next()? == "Zone" => {
            let kind = parts.next()?;
            let zone = parts.next()?;
            (Family::Vfh, kind, zone, parts.next()?.to_owned())
        }
        _ => return None
    };
    let kind = match (family, kind) {
        (_, "Orf") => ZoneKind::Socket,
        (_, "Pen") => ZoneKind::Plug,
        (Family::Ogb | Family::Vfh, "Touch") => ZoneKind::Touch,
        _ => return None
    };
    if zone.is_empty() || key.is_empty() {
        return None;
    }
    Some((ZoneId { family, kind, name: zone.to_owned() }, key))
}

/// A parameter's value. Contact proximities are floats and "close" flags are bools; a float key
/// that arrives as a bool doesn't count as a level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool)
}

/// What a zone can feel. Which of these a source listens to is its "Reacts to" switches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Contact {
    OwnHands,
    OtherHands,
    /// A socket: your own plugs. A plug: your own sockets.
    Mine,
    /// A socket: other people's plugs. A plug: other people's sockets.
    Others,
    /// Rubbing: against other people's sockets (on a socket) or plugs (on a plug).
    Rubbing
}

#[derive(Debug, Default)]
pub struct Zone {
    values: HashMap<String, Value>,
    own_plug: PlugLength,
    other_plug: PlugLength
}

impl Zone {
    pub fn set(&mut self, key: &str, value: Value) {
        self.values.insert(key.to_owned(), value);
        for (own, plug) in [("Self", &mut self.own_plug), ("Others", &mut self.other_plug)] {
            if key == format!("Pen{own}NewRoot") || key == format!("Pen{own}NewTip") {
                let root = number(&self.values, &format!("Pen{own}NewRoot"));
                let tip = number(&self.values, &format!("Pen{own}NewTip"));
                plug.update(root, tip);
            }
        }
    }

    fn number(&self, key: &str) -> Option<f64> {
        number(&self.values, key)
    }

    /// A "close" flag: on when it's any non-zero value, off when it's missing.
    fn close(&self, key: &str) -> bool {
        match self.values.get(key) {
            Some(Value::Bool(on)) => *on,
            Some(Value::Number(value)) => *value != 0.0,
            None => false
        }
    }

    fn hands(&self, own: bool) -> f64 {
        let who = if own { "Self" } else { "Others" };
        if self.close(&format!("Touch{who}Close")) { self.number(&format!("Touch{who}")).unwrap_or(0.0) } else { 0.0 }
    }

    /// The depth of a new SPS plug in this socket, if one is near.
    fn new_plug(&self, own: bool) -> Option<f64> {
        let (who, plug) = if own { ("Self", &self.own_plug) } else { ("Others", &self.other_plug) };
        depth::depth(self.number(&format!("Pen{who}NewRoot")), self.number(&format!("Pen{who}NewTip")), plug.length())
    }

    /// What this zone reads for `contact`, before any shaping.
    pub fn read(&self, family: Family, kind: ZoneKind, contact: Contact) -> f64 {
        let value = match (family, kind, contact) {
            (Family::Tps, ZoneKind::Socket, Contact::Others) => self.number("Depth_In"),
            (Family::Tps, ZoneKind::Plug, Contact::Others) => self.number("RootRoot"),
            (Family::Tps, _, _) => None,

            (_, ZoneKind::Socket | ZoneKind::Plug, Contact::OwnHands) => Some(self.hands(true)),
            (_, ZoneKind::Socket | ZoneKind::Plug, Contact::OtherHands) => Some(self.hands(false)),

            (_, ZoneKind::Socket, Contact::Mine) => self.new_plug(true).or_else(|| self.number("PenSelf")),
            (_, ZoneKind::Socket, Contact::Others) => self.new_plug(false).or_else(|| {
                // Older prefabs have no close flag: then the value counts as is.
                let close = !self.values.contains_key("PenOthersClose") || self.close("PenOthersClose");
                close.then(|| self.number("PenOthers")).flatten()
            }),
            (_, ZoneKind::Socket, Contact::Rubbing) => self.number("FrotOthers"),

            (_, ZoneKind::Plug, Contact::Mine) => self.number("PenSelf"),
            (_, ZoneKind::Plug, Contact::Others) => self.number("PenOthers"),
            (_, ZoneKind::Plug, Contact::Rubbing) => self.close("FrotOthersClose").then(|| self.number("FrotOthers")).flatten(),

            (_, ZoneKind::Touch, Contact::OwnHands) => self.number("Self"),
            (_, ZoneKind::Touch, Contact::OtherHands) => self.number("Others"),
            (_, ZoneKind::Touch, _) => None
        };
        value.unwrap_or(0.0)
    }

    /// The strongest contact right now, whatever the sources listen to: what the page shows on
    /// the zone's chip.
    pub fn strongest(&self, family: Family, kind: ZoneKind) -> f64 {
        [Contact::OwnHands, Contact::OtherHands, Contact::Mine, Contact::Others, Contact::Rubbing]
            .into_iter()
            .map(|contact| self.read(family, kind, contact))
            .fold(0.0, f64::max)
            .clamp(0.0, 1.0)
    }
}

fn number(values: &HashMap<String, Value>, key: &str) -> Option<f64> {
    match values.get(key) {
        Some(Value::Number(value)) => Some(*value),
        _ => None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(family: Family, kind: ZoneKind, name: &str) -> ZoneId {
        ZoneId { family, kind, name: name.into() }
    }

    #[test]
    fn parses_every_family() {
        assert_eq!(
            parse("OGB/Orf/Vagina/PenOthersNewTip"),
            Some((id(Family::Ogb, ZoneKind::Socket, "Vagina"), "PenOthersNewTip".into()))
        );
        assert_eq!(parse("OGB/Pen/Penis/Some/Deep/Key"), Some((id(Family::Ogb, ZoneKind::Plug, "Penis"), "Some/Deep/Key".into())));
        assert_eq!(parse("VFH/Zone/Touch/Head/Others"), Some((id(Family::Vfh, ZoneKind::Touch, "Head"), "Others".into())));
        assert_eq!(parse("TPS_Internal/Orf/Mouth/Depth_In"), Some((id(Family::Tps, ZoneKind::Socket, "Mouth"), "Depth_In".into())));
        assert_eq!(parse("TPS_Internal/Touch/Head/Others"), None);
        assert_eq!(parse("VFH/Other/Touch/Head/Others"), None);
        assert_eq!(parse("OGB/Orf/Vagina"), None);
        assert_eq!(parse("OGB/Tail/Wag/Speed"), None);
        assert_eq!(parse("TKOSC/Panic"), None);
    }

    fn zone(values: &[(&str, Value)]) -> Zone {
        let mut zone = Zone::default();
        for (key, value) in values {
            zone.set(key, *value);
        }
        zone
    }

    use Value::{Bool, Number};

    #[test]
    fn hands_need_their_close_flag() {
        let open = zone(&[("TouchOthers", Number(0.4))]);
        assert_eq!(open.read(Family::Ogb, ZoneKind::Socket, Contact::OtherHands), 0.0);
        let close = zone(&[("TouchOthers", Number(0.4)), ("TouchOthersClose", Bool(true)), ("TouchSelf", Number(0.3))]);
        assert_eq!(close.read(Family::Ogb, ZoneKind::Socket, Contact::OtherHands), 0.4);
        assert_eq!(close.read(Family::Ogb, ZoneKind::Plug, Contact::OtherHands), 0.4);
        assert_eq!(close.read(Family::Ogb, ZoneKind::Socket, Contact::OwnHands), 0.0);
    }

    #[test]
    fn socket_reads_new_plugs_then_the_old_value() {
        let old = zone(&[("PenOthers", Number(0.6))]);
        assert_eq!(old.read(Family::Ogb, ZoneKind::Socket, Contact::Others), 0.6);
        let old_far = zone(&[("PenOthers", Number(0.6)), ("PenOthersClose", Bool(false))]);
        assert_eq!(old_far.read(Family::Ogb, ZoneKind::Socket, Contact::Others), 0.0);

        let mut new = zone(&[("PenOthers", Number(0.6))]);
        for _ in 0..3 {
            new.set("PenOthersNewRoot", Number(0.5));
            new.set("PenOthersNewTip", Number(0.7));
        }
        assert_eq!(new.read(Family::Ogb, ZoneKind::Socket, Contact::Others), 0.0, "near, not in");
        new.set("PenOthersNewTip", Number(1.0));
        new.set("PenOthersNewRoot", Number(0.9));
        let depth = new.read(Family::Ogb, ZoneKind::Socket, Contact::Others);
        assert!((depth - 0.5).abs() < 1e-9, "{depth}");
        assert_eq!(new.read(Family::Ogb, ZoneKind::Socket, Contact::Mine), 0.0);
    }

    #[test]
    fn rubbing_differs_between_sockets_and_plugs() {
        let rub = zone(&[("FrotOthers", Number(0.5))]);
        assert_eq!(rub.read(Family::Ogb, ZoneKind::Socket, Contact::Rubbing), 0.5);
        assert_eq!(rub.read(Family::Ogb, ZoneKind::Plug, Contact::Rubbing), 0.0);
        let close = zone(&[("FrotOthers", Number(0.5)), ("FrotOthersClose", Bool(true))]);
        assert_eq!(close.read(Family::Ogb, ZoneKind::Plug, Contact::Rubbing), 0.5);
    }

    #[test]
    fn plugs_touch_zones_and_tps() {
        let plug = zone(&[("PenOthers", Number(0.7)), ("PenSelf", Number(0.2))]);
        assert_eq!(plug.read(Family::Ogb, ZoneKind::Plug, Contact::Others), 0.7);
        assert_eq!(plug.read(Family::Ogb, ZoneKind::Plug, Contact::Mine), 0.2);

        let touch = zone(&[("Others", Number(0.4)), ("Self", Number(0.1))]);
        assert_eq!(touch.read(Family::Vfh, ZoneKind::Touch, Contact::OtherHands), 0.4);
        assert_eq!(touch.read(Family::Vfh, ZoneKind::Touch, Contact::OwnHands), 0.1);
        assert_eq!(touch.strongest(Family::Vfh, ZoneKind::Touch), 0.4);

        let tps = zone(&[("Depth_In", Number(0.3)), ("RootRoot", Number(0.8))]);
        assert_eq!(tps.read(Family::Tps, ZoneKind::Socket, Contact::Others), 0.3);
        assert_eq!(tps.read(Family::Tps, ZoneKind::Plug, Contact::Others), 0.8);
        assert_eq!(tps.read(Family::Tps, ZoneKind::Socket, Contact::OtherHands), 0.0);
    }

    #[test]
    fn a_bool_isnt_a_level() {
        let zone = zone(&[("Others", Bool(true))]);
        assert_eq!(zone.read(Family::Ogb, ZoneKind::Touch, Contact::OtherHands), 0.0);
    }
}
