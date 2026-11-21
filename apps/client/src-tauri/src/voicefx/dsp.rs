//! The effects themselves, one mono sample at a time. Turning one on or off fades over a moment
//! so nothing clicks; with none on, the voice goes through untouched.
use std::f32::consts::TAU;

use super::model::Effect;

/// How often filter settings follow the strength and the wobble, in samples.
const RETUNE_EVERY: u32 = 32;
/// The fade when an effect starts or stops, and when its strength moves.
const FADE_SECONDS: f32 = 0.03;

#[derive(Debug, Default, Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32
}

impl Biquad {
    /// A 2-pole low-pass (or with `high`, high-pass) at `cutoff` Hz.
    fn tune(&mut self, cutoff: f32, rate: f32, high: bool) {
        let w = TAU * (cutoff / rate).clamp(0.001, 0.45);
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2.0 * 0.707);
        let a0 = 1.0 + alpha;
        let edge = if high { (1.0 + cos) / 2.0 } else { (1.0 - cos) / 2.0 };
        self.b0 = edge / a0;
        self.b1 = (if high { -(1.0 + cos) } else { 1.0 - cos }) / a0;
        self.b2 = edge / a0;
        self.a1 = -2.0 * cos / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

pub struct Dsp {
    rate: f32,
    /// The effect whose filters are set up: kept while it fades out.
    effect: Effect,
    /// 0 clean, 1 fully the effect.
    wet: f32,
    level: f32,
    fade: f32,
    low: [Biquad; 2],
    high: Biquad,
    gain: f32,
    drive: f32,
    hiss: f32,
    phase: f32,
    noise: u32,
    until_retune: u32
}

impl Dsp {
    pub fn new(rate: f32) -> Self {
        Self {
            rate,
            effect: Effect::Muffled,
            wet: 0.0,
            level: 0.0,
            fade: 1.0 - (-1.0 / (FADE_SECONDS * rate)).exp(),
            low: [Biquad::default(); 2],
            high: Biquad::default(),
            gain: 1.0,
            drive: 1.0,
            hiss: 0.0,
            phase: 0.0,
            noise: 0x2545_f491,
            until_retune: 0
        }
    }

    /// From `from` at strength 0 to `to` at strength 1, evenly to the ear.
    fn sweep(&self, from: f32, to: f32) -> f32 {
        from * (to / from).powf(self.level)
    }

    fn retune(&mut self) {
        let s = self.level;
        // A slow wave for the underwater wobble.
        self.phase = (self.phase + 0.6 * RETUNE_EVERY as f32 / self.rate).fract();
        let wave = (self.phase * TAU).sin();
        let (low, high) = match self.effect {
            Effect::Muffled => {
                self.gain = 1.0 - 0.5 * s;
                (self.sweep(6000.0, 300.0), 20.0)
            }
            Effect::Underwater => {
                self.gain = (1.0 - 0.3 * s) * (1.0 + 0.2 * s * wave);
                (self.sweep(1800.0, 350.0) * (1.0 + 0.35 * wave), 20.0)
            }
            Effect::Radio => {
                self.gain = 1.0;
                (self.sweep(5000.0, 2400.0), self.sweep(200.0, 700.0))
            }
            Effect::SoftMute => {
                self.gain = 1.0 - s;
                (20000.0, 20.0)
            }
        };
        self.drive = if self.effect == Effect::Radio { 1.0 + 6.0 * s } else { 1.0 };
        self.hiss = if self.effect == Effect::Radio { 0.012 * s } else { 0.0 };
        for filter in &mut self.low {
            filter.tune(low, self.rate, false);
        }
        self.high.tune(high, self.rate, true);
    }

    /// One sample through `effect` (`None`: clean) at `level` (0 to 1).
    pub fn process(&mut self, x: f32, effect: Option<Effect>, level: f32) -> f32 {
        if let Some(effect) = effect {
            if effect != self.effect {
                self.effect = effect;
                self.until_retune = 0;
            }
        }
        let target = if effect.is_some() { 1.0 } else { 0.0 };
        self.wet += (target - self.wet) * self.fade;
        self.level += (level.clamp(0.0, 1.0) - self.level) * self.fade;
        if self.wet < 0.0005 && effect.is_none() {
            self.wet = 0.0;
            return x;
        }
        if self.until_retune == 0 {
            self.retune();
            self.until_retune = RETUNE_EVERY;
        }
        self.until_retune -= 1;

        let mut y = self.high.run(x);
        for filter in &mut self.low {
            y = filter.run(y);
        }
        if self.drive > 1.0 {
            // Overdriven like a cheap speaker, brought back to about the same loudness.
            y = (y * self.drive).tanh() / self.drive.tanh().max(0.5);
        }
        if self.hiss > 0.0 {
            self.noise ^= self.noise << 13;
            self.noise ^= self.noise >> 17;
            self.noise ^= self.noise << 5;
            y += (self.noise as f32 / u32::MAX as f32 - 0.5) * 2.0 * self.hiss;
        }
        x * (1.0 - self.wet) + y * self.gain * self.wet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48000.0;

    /// How loud a sine of `hz` comes out, after the fade has settled.
    fn loudness(effect: Option<Effect>, level: f32, hz: f32) -> f32 {
        let mut dsp = Dsp::new(RATE);
        let mut sum = 0.0;
        let total = RATE as usize;
        for n in 0..total {
            let y = dsp.process((TAU * hz * n as f32 / RATE).sin(), effect, level);
            assert!(y.is_finite());
            if n >= total / 2 {
                sum += y * y;
            }
        }
        (sum / (total / 2) as f32).sqrt()
    }

    #[test]
    fn with_nothing_on_the_voice_is_untouched() {
        let mut dsp = Dsp::new(RATE);
        for x in [0.0, 0.25, -0.8, 1.0] {
            assert_eq!(dsp.process(x, None, 1.0), x);
        }
    }

    #[test]
    fn muffled_cuts_the_highs_and_keeps_the_lows() {
        let clean = loudness(None, 1.0, 5000.0);
        assert!(loudness(Some(Effect::Muffled), 1.0, 5000.0) < clean * 0.02);
        assert!(loudness(Some(Effect::Muffled), 1.0, 150.0) > clean * 0.3);
        // Stronger means more muffled.
        assert!(loudness(Some(Effect::Muffled), 1.0, 2000.0) < loudness(Some(Effect::Muffled), 0.3, 2000.0));
    }

    #[test]
    fn soft_mute_fades_all_the_way_out() {
        assert!(loudness(Some(Effect::SoftMute), 1.0, 440.0) < 0.001);
        let half = loudness(Some(Effect::SoftMute), 0.5, 440.0) / loudness(None, 0.5, 440.0);
        assert!((half - 0.5).abs() < 0.05);
    }

    #[test]
    fn radio_is_thin_and_underwater_is_dull() {
        let clean = loudness(None, 1.0, 100.0);
        assert!(loudness(Some(Effect::Radio), 1.0, 100.0) < clean * 0.2);
        assert!(loudness(Some(Effect::Underwater), 1.0, 6000.0) < clean * 0.02);
    }

    #[test]
    fn an_effect_fades_out_when_it_ends() {
        let mut dsp = Dsp::new(RATE);
        for _ in 0..RATE as usize / 2 {
            dsp.process(0.5, Some(Effect::SoftMute), 1.0);
        }
        // Right after it ends it's still quiet, and half a second later it's clean again.
        assert!(dsp.process(0.5, None, 1.0) < 0.05);
        let mut last = 0.0;
        for _ in 0..RATE as usize / 2 {
            last = dsp.process(0.5, None, 1.0);
        }
        assert_eq!(last, 0.5);
    }
}
