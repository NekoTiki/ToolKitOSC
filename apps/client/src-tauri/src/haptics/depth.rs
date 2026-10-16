//! How deep a new SPS plug is inside a socket. Such a plug carries two contacts, at its root and its
//! tip, and the socket's receivers are 1 m spheres, so each reads like a distance in metres. While
//! the plug approaches, `tip − root` is its length; once the tip is in, depth is how much of that
//! length is no longer outside.

/// Lengths kept while the plug approaches.
const SAMPLES: usize = 8;
/// Fewer samples than this and only the fallback length counts.
const MIN_SAMPLES: usize = 4;
/// Below this either contact reads as nobody nearby.
const NOBODY: f64 = 0.01;
/// The root is too close for its value to say anything about the length.
const ROOT_TOO_CLOSE: f64 = 0.95;
/// Shorter than this isn't a plug.
const MIN_LENGTH: f64 = 0.02;
/// The tip counts as inside past this.
const TIP_INSIDE: f64 = 0.99;

/// The length of the plug near one socket, learned from its root and tip values. Samples arrive
/// when only one of the two values has updated, so a single one can be off: the length is where
/// the samples agree best.
#[derive(Debug, Default, Clone)]
pub struct PlugLength {
    /// Newest first.
    samples: Vec<f64>,
    /// The longest length seen while the tip was already inside, used until there are samples.
    inside: Option<f64>
}

impl PlugLength {
    /// Takes the latest root and tip values (either may be missing).
    pub fn update(&mut self, root: Option<f64>, tip: Option<f64>) {
        let (Some(root), Some(tip)) = (root, tip) else {
            return self.reset();
        };
        if root < NOBODY || tip < NOBODY {
            return self.reset();
        }
        let length = tip - root;
        if root > ROOT_TOO_CLOSE || length < MIN_LENGTH {
            return;
        }
        if tip > TIP_INSIDE {
            self.inside = Some(self.inside.map_or(length, |inside| inside.max(length)));
        } else {
            self.samples.insert(0, length);
            self.samples.truncate(SAMPLES);
        }
    }

    fn reset(&mut self) {
        self.samples.clear();
        self.inside = None;
    }

    pub fn length(&self) -> Option<f64> {
        if self.samples.len() < MIN_SAMPLES {
            return self.inside;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_by(f64::total_cmp);
        // The closer pair wins, the first one on a tie; its longer value is the length.
        let best = sorted.windows(2).enumerate().fold(None, |best: Option<(usize, f64)>, (at, pair)| {
            let gap = pair[1] - pair[0];
            match best {
                Some((_, best_gap)) if best_gap <= gap => best,
                _ => Some((at + 1, gap))
            }
        });
        Some(best.map_or(self.samples[0], |(at, _)| sorted[at]))
    }
}

/// How deep the plug is (0 = at the entrance), or `None` when nobody's plug is near, so the
/// caller can fall back to the older single value. Not clamped.
pub fn depth(root: Option<f64>, tip: Option<f64>, length: Option<f64>) -> Option<f64> {
    let (root, tip) = (root?, tip?);
    if root <= 0.0 && tip <= 0.0 {
        return None;
    }
    match length {
        Some(length) if length > 0.0 && tip > TIP_INSIDE => Some(1.0 - (1.0 - root) / length),
        // Near, but not in yet.
        _ => Some(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approach(length: &mut PlugLength, pairs: &[(f64, f64)]) {
        for &(root, tip) in pairs {
            length.update(Some(root), Some(tip));
        }
    }

    #[test]
    fn learns_the_length_while_approaching() {
        let mut length = PlugLength::default();
        approach(&mut length, &[(0.3, 0.5), (0.4, 0.6), (0.45, 0.65)]);
        assert_eq!(length.length(), None, "three samples aren't enough");
        approach(&mut length, &[(0.5, 0.7)]);
        let learned = length.length().unwrap();
        assert!((learned - 0.2).abs() < 1e-9, "{learned}");
    }

    #[test]
    fn an_outlier_doesnt_win() {
        let mut length = PlugLength::default();
        // One value updated before the other: 0.35 is off.
        approach(&mut length, &[(0.3, 0.5), (0.3, 0.65), (0.4, 0.6), (0.5, 0.7), (0.55, 0.751)]);
        let learned = length.length().unwrap();
        assert!((learned - 0.2).abs() < 1e-9, "{learned}");
    }

    #[test]
    fn falls_back_to_the_longest_length_seen_inside() {
        let mut length = PlugLength::default();
        approach(&mut length, &[(0.8, 0.995), (0.85, 0.995), (0.82, 0.995)]);
        let learned = length.length().unwrap();
        assert!((learned - 0.195).abs() < 1e-9, "{learned}");
    }

    #[test]
    fn ignores_useless_samples_and_resets_when_nobody_is_near() {
        let mut length = PlugLength::default();
        approach(&mut length, &[(0.3, 0.5), (0.4, 0.6), (0.45, 0.65), (0.5, 0.7)]);
        approach(&mut length, &[(0.96, 0.995), (0.5, 0.51)]);
        assert!((length.length().unwrap() - 0.2).abs() < 1e-9);
        length.update(Some(0.0), Some(0.3));
        assert_eq!(length.length(), None);
        approach(&mut length, &[(0.3, 0.5)]);
        length.update(None, Some(0.5));
        assert_eq!(length.length(), None);
    }

    #[test]
    fn depth_from_root_once_the_tip_is_in() {
        assert_eq!(depth(None, Some(0.5), Some(0.2)), None);
        assert_eq!(depth(Some(0.0), Some(0.0), Some(0.2)), None);
        assert_eq!(depth(Some(0.7), Some(0.9), Some(0.2)), Some(0.0));
        assert_eq!(depth(Some(0.9), Some(1.0), None), Some(0.0));
        let half = depth(Some(0.9), Some(1.0), Some(0.2)).unwrap();
        assert!((half - 0.5).abs() < 1e-9, "{half}");
        assert!((depth(Some(1.0), Some(1.0), Some(0.2)).unwrap() - 1.0).abs() < 1e-9);
    }
}
