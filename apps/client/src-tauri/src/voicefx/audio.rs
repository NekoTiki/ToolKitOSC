//! The audio path: the microphone in, through `dsp`, out to the cable VRChat listens to (and, for
//! testing, to your own headphones). What plays is set through `Shared`, without stopping anything.
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::dsp::Dsp;
use super::model::Effect;

// shortcut: every device is opened at 48 kHz and the system converts (WASAPI and PipeWire do).
// Add a resampler if a backend refuses.
const RATE: u32 = 48_000;
/// The two devices' clocks drift apart: past this much waiting audio, the oldest is dropped.
const MAX_QUEUED: usize = RATE as usize / 5;
const KEEP_QUEUED: usize = RATE as usize / 20;

/// What the audio callbacks read and report, lock-free.
#[derive(Default)]
pub struct Shared {
    /// 0: clean. Otherwise the effect's place in `Effect::ALL`, from 1.
    effect: AtomicU8,
    level: AtomicU32,
    /// The loudest sample since the last `take_peak`.
    peak: AtomicU32,
    /// A stream stopped by itself: a device was unplugged.
    failed: AtomicBool
}

impl Shared {
    pub fn play(&self, effect: Option<Effect>, level: f64) {
        let index = effect.map_or(0, |effect| Effect::ALL.iter().position(|e| *e == effect).unwrap_or(0) as u8 + 1);
        self.level.store((level as f32).to_bits(), Ordering::Relaxed);
        self.effect.store(index, Ordering::Relaxed);
    }

    fn playing(&self) -> (Option<Effect>, f32) {
        let index = self.effect.load(Ordering::Relaxed) as usize;
        (index.checked_sub(1).and_then(|i| Effect::ALL.get(i).copied()), f32::from_bits(self.level.load(Ordering::Relaxed)))
    }

    /// How loud the microphone got since the last call, 0 to 1.
    pub fn take_peak(&self) -> f32 {
        f32::from_bits(self.peak.swap(0, Ordering::Relaxed)).min(1.0)
    }

    pub fn take_failed(&self) -> bool {
        self.failed.swap(false, Ordering::Relaxed)
    }
}

/// The open streams. Dropping it closes them.
pub struct Running {
    _streams: Vec<cpal::Stream>
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct Devices {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>
}

fn name(device: &cpal::Device) -> Option<String> {
    device.description().ok().map(|description| description.name().to_owned())
}

pub fn devices() -> Devices {
    let host = cpal::default_host();
    let names = |devices: Option<Vec<cpal::Device>>| {
        let mut names: Vec<String> = devices.into_iter().flatten().filter_map(|device| name(&device)).collect();
        names.sort();
        names.dedup();
        names
    };
    Devices {
        inputs: names(host.input_devices().ok().map(Iterator::collect)),
        outputs: names(host.output_devices().ok().map(Iterator::collect))
    }
}

pub fn has_input(wanted: &str) -> bool {
    cpal::default_host().input_devices().is_ok_and(|mut devices| devices.any(|device| name(&device).as_deref() == Some(wanted)))
}

type Queue = Arc<Mutex<VecDeque<f32>>>;

fn output_stream(device: &cpal::Device, shared: &Arc<Shared>) -> Result<(cpal::Stream, Queue), String> {
    let channels = device.default_output_config().map_err(|err| err.to_string())?.channels();
    let config = cpal::StreamConfig { channels, sample_rate: RATE, buffer_size: cpal::BufferSize::Default };
    let queue: Queue = Arc::default();
    let (reader, failed) = (queue.clone(), shared.clone());
    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut queue = reader.lock().unwrap();
                for frame in data.chunks_mut(channels as usize) {
                    // Nothing waiting yet: silence, never a stale sample.
                    frame.fill(queue.pop_front().unwrap_or(0.0));
                }
            },
            move |err| {
                tracing::warn!("Voice effects output stopped: {err}");
                failed.failed.store(true, Ordering::Relaxed);
            },
            None
        )
        .map_err(|err| err.to_string())?;
    Ok((stream, queue))
}

/// Opens the microphone (`None`: the default one) and the cable, and starts passing your voice
/// through.
pub fn start(mic: Option<&str>, output: &str, monitor: bool, shared: &Arc<Shared>) -> Result<Running, String> {
    let host = cpal::default_host();
    let input = match mic {
        Some(wanted) => host.input_devices().map_err(|err| err.to_string())?.find(|device| name(device).as_deref() == Some(wanted)),
        None => host.default_input_device()
    }
    .ok_or("The microphone wasn't found")?;
    let cable = host
        .output_devices()
        .map_err(|err| err.to_string())?
        .find(|device| name(device).as_deref() == Some(output))
        .ok_or("The output wasn't found")?;

    let (cable_stream, cable_queue) = output_stream(&cable, shared).map_err(|err| format!("Couldn't open the output: {err}"))?;
    let mut queues = vec![cable_queue];
    let mut streams = vec![cable_stream];
    if monitor {
        // Hearing yourself is a nicety: your voice still goes out without it.
        match host.default_output_device().ok_or("no default output".to_owned()).and_then(|device| output_stream(&device, shared)) {
            Ok((stream, queue)) => {
                streams.push(stream);
                queues.push(queue);
            }
            Err(err) => tracing::warn!("Couldn't open your headphones to hear yourself: {err}")
        }
    }

    let channels = input.default_input_config().map_err(|err| format!("Couldn't open the microphone: {err}"))?.channels() as usize;
    let config = cpal::StreamConfig { channels: channels as u16, sample_rate: RATE, buffer_size: cpal::BufferSize::Default };
    let mut dsp = Dsp::new(RATE as f32);
    let mut processed: Vec<f32> = Vec::new();
    let (reader, failed) = (shared.clone(), shared.clone());
    let input_stream = input
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let (effect, level) = reader.playing();
                let mut peak = 0f32;
                processed.clear();
                for frame in data.chunks(channels) {
                    let x = frame.iter().sum::<f32>() / channels as f32;
                    peak = peak.max(x.abs());
                    processed.push(dsp.process(x, effect, level));
                }
                // Positive floats order the same as their bits.
                reader.peak.fetch_max(peak.to_bits(), Ordering::Relaxed);
                for queue in &queues {
                    let mut queue = queue.lock().unwrap();
                    queue.extend(processed.iter());
                    if queue.len() > MAX_QUEUED {
                        let extra = queue.len() - KEEP_QUEUED;
                        queue.drain(..extra);
                    }
                }
            },
            move |err| {
                tracing::warn!("Voice effects microphone stopped: {err}");
                failed.failed.store(true, Ordering::Relaxed);
            },
            None
        )
        .map_err(|err| format!("Couldn't open the microphone: {err}"))?;
    streams.push(input_stream);

    for stream in &streams {
        stream.play().map_err(|err| err.to_string())?;
    }
    Ok(Running { _streams: streams })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Opens this machine's real devices for a moment: `cargo test --lib real_devices -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_devices_open_and_pass_audio() {
        let found = devices();
        println!("inputs: {:?}\noutputs: {:?}", found.inputs, found.outputs);
        let output = found.outputs.iter().find(|name| name.to_lowercase().contains("cable input")).or(found.outputs.first()).expect("an output device");
        let shared = Arc::new(Shared::default());
        shared.play(Some(Effect::Muffled), 0.7);
        let running = start(None, output, false, &shared).expect("the streams open");
        std::thread::sleep(std::time::Duration::from_millis(600));
        println!("opened {output}, peak {}", shared.take_peak());
        assert!(!shared.take_failed());
        drop(running);
    }
}
