//! Voice effects: your microphone goes through the app before VRChat hears it, so an effect
//! (muffled, underwater, radio, soft mute) can be put on your voice. `audio` carries the sound,
//! `dsp` is the effects, `holds` decides which one plays, and `voicefx.json` (see `store`) keeps
//! the devices and defaults.
//!
//! Your avatar, viewers' controls, automations, the mouth socket link and you on the page each
//! hold an effect on; the newest hold plays, one at a time. Avatar parameters: `TKOSC/VoiceFX`
//! (bool, in) holds one from your menu or a contact, `TKOSC/VoiceFXEffect` (int, in) and
//! `TKOSC/VoiceFXLevel` (float, in) say which and how strong, and `TKOSC/VoiceFXOn` (bool, out)
//! is true while any effect plays, whoever started it. `TKOSC/Panic` and Stop everything end
//! every hold; your voice keeps going through, clean.
mod audio;
pub mod control;
mod dsp;
mod holds;
pub mod model;
mod store;
pub mod commands;

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

use crate::automations::model::{Action, VoiceFxEvent, VoiceFxRun, VoiceFxSource};
use crate::haptics::zones::{self, Contact, ZoneId, ZoneKind};
use crate::state::AppState;
use crate::types::{AvatarDetails, OscArg, OscCommand, OscMessage};
use holds::{Holds, Source};
use model::{Effect, MicMode, Settings};

/// While something is held or in the mouth socket.
const TICK: Duration = Duration::from_millis(50);
/// Otherwise: the level meter and the devices to keep up. A new hold wakes the loop right away.
const IDLE: Duration = Duration::from_millis(100);
const VIEW_EVERY: Duration = Duration::from_millis(100);
/// How often a device that couldn't be opened, or a missing VR mic, is tried again.
const RETRY_EVERY: Duration = Duration::from_secs(3);
/// A socket counts as empty under this depth.
const INSIDE: f64 = 0.02;

const SWITCH_PARAMETER: &str = "TKOSC/VoiceFX";
const EFFECT_PARAMETER: &str = "TKOSC/VoiceFXEffect";
const LEVEL_PARAMETER: &str = "TKOSC/VoiceFXLevel";
const ON_PARAMETER: &str = "TKOSC/VoiceFXOn";
const PANIC_PARAMETER: &str = "TKOSC/Panic";

#[derive(Default)]
pub struct VoiceFx {
    /// `None` until read from disk (see `store`).
    settings: Mutex<Option<Settings>>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    wake: Notify,
    run: Mutex<Run>,
    shared: Arc<audio::Shared>,
    audio: Mutex<Audio>,
    /// The last `voicefx-state` sent, and when.
    sent: Mutex<Option<(Instant, View)>>
}

#[derive(Default)]
struct Run {
    holds: Holds,
    avatar: AvatarInput,
    /// The avatar's sockets, as their contacts last read.
    sockets: HashMap<ZoneId, zones::Zone>,
    /// Stop everything ended the mouth socket's hold: it starts again once the socket is empty.
    mouth_stopped: bool,
    /// When something was last in the mouth socket.
    mouth_inside: Option<Instant>,
    /// When each timed control can be used again.
    cooldowns: HashMap<String, Instant>,
    /// What plays, as automations and the avatar last heard it.
    playing: Option<(Effect, VoiceFxSource)>,
    on_sent: Option<bool>,
    status: String
}

/// What the avatar's own parameters last said.
#[derive(Default)]
struct AvatarInput {
    effect: Option<Effect>,
    level: Option<f64>,
    /// Stop everything ended its hold: `TKOSC/VoiceFX` has to go off and on again.
    stopped: bool
}

#[derive(Default)]
struct Audio {
    running: Option<audio::Running>,
    /// The settings the streams were last opened (or tried) with.
    wanted: Option<Wanted>,
    error: Option<String>,
    /// The microphone in use. `None`: the system's default.
    mic: Option<String>,
    /// The VR mic is missing, so the desktop one is used.
    fallback: bool,
    retry_at: Option<Instant>
}

#[derive(Clone, PartialEq)]
struct Wanted {
    mic: Option<String>,
    mic_vr: Option<String>,
    vr: bool,
    output: Option<String>,
    monitor: bool
}

fn voicefx(app: &AppHandle) -> &VoiceFx {
    &app.state::<AppState>().inner().voicefx
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldView {
    pub source: Source,
    pub by: Option<String>,
    pub effect: Effect,
    pub level: f64,
    /// When a timed hold ends, in ms since 1970.
    pub ends_at: Option<u64>,
    pub playing: bool
}

/// A Voice effects control's tile, here and on the share page.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlView {
    pub on: bool,
    pub by: Option<String>,
    pub effect: Option<Effect>,
    pub level: Option<f64>,
    pub ends_at: Option<u64>,
    /// When a timed control can be used again, in ms since 1970.
    pub cooldown_until: Option<u64>
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SocketView {
    pub name: String,
    pub depth: f64
}

/// What the Voice effects page shows.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub holds: Vec<HoldView>,
    pub controls: BTreeMap<String, ControlView>,
    /// Stop everything is on.
    pub paused: bool,
    pub running: bool,
    pub error: Option<String>,
    pub mic: Option<String>,
    pub mic_fallback: bool,
    pub vr: bool,
    /// How loud the microphone is, 0 to 1.
    pub peak: f64,
    pub sockets: Vec<SocketView>,
    /// The avatar's sockets the mouth link follows.
    pub mouth_sockets: Vec<String>
}

fn epoch_ms(at: Instant) -> u64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let (now_instant, ms) = (Instant::now(), now.as_millis() as u64);
    match at.checked_duration_since(now_instant) {
        Some(ahead) => ms + ahead.as_millis() as u64,
        None => ms.saturating_sub(now_instant.duration_since(at).as_millis() as u64)
    }
}

fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn source_kind(hold: &holds::Hold) -> VoiceFxSource {
    match hold.source {
        Source::You => VoiceFxSource::You,
        Source::Avatar => VoiceFxSource::Avatar,
        Source::Mouth => VoiceFxSource::Mouth,
        Source::Automation => VoiceFxSource::Automation,
        // A control you clicked yourself is you.
        Source::Control(_) if hold.by.is_some() => VoiceFxSource::Viewer,
        Source::Control(_) => VoiceFxSource::You
    }
}

/// How deep the contacts the mouth link reacts to are in `zone`, 0 to 1.
fn depth(id: &ZoneId, zone: &zones::Zone, mouth: &model::Mouth) -> f64 {
    let contacts = [(mouth.others, Contact::Others), (mouth.own, Contact::Mine), (mouth.hands, Contact::OtherHands), (mouth.hands, Contact::OwnHands)];
    contacts
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, contact)| zone.read(id.family, id.kind, contact))
        .fold(0.0, f64::max)
        .clamp(0.0, 1.0)
}

/// Every socket on the avatar and how deep something is in it, by name.
fn sockets(run: &Run, mouth: &model::Mouth) -> Vec<SocketView> {
    let mut depths: BTreeMap<&str, f64> = BTreeMap::new();
    for (id, zone) in &run.sockets {
        let entry = depths.entry(id.name.as_str()).or_default();
        *entry = entry.max(depth(id, zone, mouth));
    }
    depths.into_iter().map(|(name, depth)| SocketView { name: name.to_owned(), depth: round(depth) }).collect()
}

/// The sockets the mouth link follows, of `sockets`.
fn followed<'a>(sockets: &'a [SocketView], mouth: &'a model::Mouth) -> impl Iterator<Item = &'a SocketView> {
    sockets.iter().filter(|socket| if mouth.custom { mouth.sockets.contains(&socket.name) } else { model::looks_like_mouth(&socket.name) })
}

pub fn view(app: &AppHandle) -> View {
    let settings = store::get(app);
    let v = voicefx(app);
    let run = v.run.lock().unwrap();
    let audio = v.audio.lock().unwrap();
    let now = Instant::now();

    let holds: Vec<HoldView> = run
        .holds
        .newest_first()
        .into_iter()
        .enumerate()
        .map(|(rank, hold)| HoldView {
            source: hold.source.clone(),
            by: hold.by.clone(),
            effect: hold.effect.unwrap_or(settings.default_effect),
            level: round(hold.level),
            ends_at: hold.until.map(epoch_ms),
            playing: rank == 0
        })
        .collect();

    let mut controls: BTreeMap<String, ControlView> = BTreeMap::new();
    for hold in &holds {
        if let Source::Control(id) = &hold.source {
            controls.insert(
                id.clone(),
                ControlView { on: true, by: hold.by.clone(), effect: Some(hold.effect), level: Some(hold.level), ends_at: hold.ends_at, cooldown_until: None }
            );
        }
    }
    for (id, until) in &run.cooldowns {
        if *until > now {
            let entry = controls
                .entry(id.clone())
                .or_insert(ControlView { on: false, by: None, effect: None, level: None, ends_at: None, cooldown_until: None });
            entry.cooldown_until = Some(epoch_ms(*until));
        }
    }

    let sockets = sockets(&run, &settings.mouth);
    View {
        holds,
        controls,
        paused: crate::automations::is_paused(app),
        running: audio.running.is_some(),
        error: audio.error.clone(),
        mic: audio.mic.clone(),
        mic_fallback: audio.fallback,
        vr: settings.last_vr,
        peak: f64::from((v.shared.take_peak() * 20.0).round() / 20.0),
        mouth_sockets: followed(&sockets, &settings.mouth).map(|socket| socket.name.clone()).collect(),
        sockets
    }
}

/// A live avatar parameter from VRChat. Values read when an avatar loads never get here, so an
/// avatar never loads with an effect on.
pub fn on_parameter(app: &AppHandle, message: &OscMessage) {
    let Some(name) = message.address.strip_prefix("/avatar/parameters/") else {
        return;
    };
    let value = match message.args.first() {
        Some(OscArg::Number(value)) => zones::Value::Number(*value),
        Some(OscArg::Bool(on)) => zones::Value::Bool(*on),
        _ => return
    };
    let v = voicefx(app);
    if v.task.lock().unwrap().is_none() {
        return;
    }
    let number = match value {
        zones::Value::Number(value) => value,
        zones::Value::Bool(on) => f64::from(u8::from(on))
    };

    match name {
        SWITCH_PARAMETER if number != 0.0 => {
            let mut run = v.run.lock().unwrap();
            // After Stop everything, only a toggle that went off first starts it again.
            if !crate::automations::is_paused(app) && !run.avatar.stopped {
                // What it plays follows the settings and the other two parameters every tick.
                let (effect, level) = (run.avatar.effect, run.avatar.level.unwrap_or(0.0));
                run.holds.hold(Source::Avatar, None, effect, level, None);
            }
        }
        SWITCH_PARAMETER => {
            let mut run = v.run.lock().unwrap();
            run.avatar.stopped = false;
            run.holds.release(&Source::Avatar);
        }
        EFFECT_PARAMETER => v.run.lock().unwrap().avatar.effect = Effect::from_parameter(number),
        LEVEL_PARAMETER => v.run.lock().unwrap().avatar.level = Some(number.clamp(0.0, 1.0)),
        // The webview's Stop everything follows; this ends every hold even before.
        PANIC_PARAMETER if number != 0.0 => stop_holds(app),
        _ => {
            let Some((id, key)) = zones::parse(name) else {
                return;
            };
            if id.kind != ZoneKind::Socket {
                return;
            }
            v.run.lock().unwrap().sockets.entry(id).or_default().set(&key, value);
        }
    }
    v.wake.notify_one();
}

/// The avatar's sockets from its OSC config, so they show before anything touches them.
fn add_sockets(run: &mut Run, details: Option<&AvatarDetails>) {
    for parameter in details.into_iter().flat_map(|details| &details.parameters) {
        if let Some((id, _)) = zones::parse(&parameter.name).filter(|(id, _)| id.kind == ZoneKind::Socket) {
            run.sockets.entry(id).or_default();
        }
    }
}

/// Another avatar: its sockets start from its config, and it hears whether an effect plays.
pub fn avatar_changed(app: &AppHandle, details: &AvatarDetails) {
    let v = voicefx(app);
    {
        let mut run = v.run.lock().unwrap();
        run.sockets.clear();
        add_sockets(&mut run, Some(details));
        run.avatar = AvatarInput::default();
        run.holds.release(&Source::Avatar);
        run.holds.release(&Source::Mouth);
        run.on_sent = None;
    }
    v.wake.notify_one();
}

/// Ends every hold. Your avatar's toggle and the mouth socket have to let go before they start
/// one again.
fn stop_holds(app: &AppHandle) {
    let v = voicefx(app);
    {
        let mut run = v.run.lock().unwrap();
        run.avatar.stopped = run.holds.is_held(&Source::Avatar);
        run.mouth_stopped = run.holds.is_held(&Source::Mouth);
        run.holds.clear();
    }
    v.wake.notify_one();
}

fn paused_error(app: &AppHandle) -> Result<(), String> {
    if crate::automations::is_paused(app) {
        return Err("Stop everything is on. Resume first.".into());
    }
    Ok(())
}

/// Turn on myself, on the page.
pub fn hold_mine(app: &AppHandle, on: bool) -> Result<(), String> {
    let v = voicefx(app);
    if on {
        paused_error(app)?;
        let level = store::get(app).level;
        v.run.lock().unwrap().holds.hold(Source::You, None, None, level, None);
    } else {
        v.run.lock().unwrap().holds.release(&Source::You);
    }
    v.wake.notify_one();
    Ok(())
}

/// The X on a hold, on the page.
pub fn release(app: &AppHandle, source: &Source) {
    let v = voicefx(app);
    {
        let mut run = v.run.lock().unwrap();
        run.holds.release(source);
        // Like Stop everything, for that one source.
        match source {
            Source::Avatar => run.avatar.stopped = true,
            Source::Mouth => run.mouth_stopped = true,
            _ => {}
        }
    }
    v.wake.notify_one();
}

/// An automation's Voice effects step.
pub fn run_step(app: &AppHandle, action: &Action) {
    let Action::VoiceFx { run, effect, level, seconds } = action else {
        return;
    };
    let v = voicefx(app);
    if v.task.lock().unwrap().is_none() {
        return;
    }
    {
        let mut state = v.run.lock().unwrap();
        let held = state.holds.is_held(&Source::Automation);
        let on = match run {
            VoiceFxRun::On => true,
            VoiceFxRun::Off => false,
            VoiceFxRun::Toggle => !held
        };
        if on && !crate::automations::is_paused(app) {
            let until = (*seconds > 0).then(|| Instant::now() + Duration::from_secs(u64::from(*seconds)));
            state.holds.hold(Source::Automation, None, *effect, *level, until);
        } else {
            state.holds.release(&Source::Automation);
        }
    }
    v.wake.notify_one();
}

/// Opens, reopens or closes the audio streams so they match the settings.
fn keep_audio(app: &AppHandle, settings: &Settings, now: Instant) {
    let v = voicefx(app);
    let mut audio = v.audio.lock().unwrap();
    let wanted = Wanted {
        mic: settings.mic.clone(),
        mic_vr: (settings.mic_mode == MicMode::Split).then(|| settings.mic_vr.clone()).flatten(),
        vr: settings.mic_mode == MicMode::Split && settings.last_vr,
        output: settings.output.clone(),
        monitor: settings.monitor
    };
    let failed = v.shared.take_failed();
    let retry = audio.retry_at.is_some_and(|at| now >= at);
    if audio.wanted.as_ref() == Some(&wanted) && !failed && !retry {
        return;
    }
    // A retry only looks for the VR mic while the desktop one stands in for it.
    let vr_mic = wanted.mic_vr.as_ref().filter(|_| wanted.vr);
    if retry && audio.running.is_some() && !failed && audio.wanted.as_ref() == Some(&wanted) {
        audio.retry_at = Some(now + RETRY_EVERY);
        if !vr_mic.is_some_and(|mic| audio::has_input(mic)) {
            return;
        }
    }

    audio.running = None;
    audio.wanted = Some(wanted.clone());
    audio.retry_at = None;
    audio.fallback = false;
    let Some(output) = &wanted.output else {
        audio.error = Some("Pick the output VRChat listens to.".into());
        return;
    };
    let mic = match vr_mic {
        Some(mic) if audio::has_input(mic) => Some(mic.clone()),
        Some(_) => {
            audio.fallback = true;
            audio.retry_at = Some(now + RETRY_EVERY);
            wanted.mic.clone()
        }
        None => wanted.mic.clone()
    };
    match audio::start(mic.as_deref(), output, wanted.monitor, &v.shared) {
        Ok(running) => {
            tracing::info!("Voice effects: {} -> {output}", mic.as_deref().unwrap_or("default microphone"));
            audio.running = Some(running);
            audio.error = None;
        }
        Err(err) => {
            tracing::warn!("Voice effects couldn't start: {err}");
            audio.error = Some(err);
            audio.retry_at = Some(now + RETRY_EVERY);
        }
    }
    audio.mic = mic;
}

/// One tick. True while something is held or in a socket, so the next one comes soon.
fn tick(app: &AppHandle) -> bool {
    let now = Instant::now();
    let v = voicefx(app);
    let mut settings = store::get(app);

    // VRChat says whether you're in VR when you load in: the mic follows.
    let vr = {
        let state = app.state::<AppState>();
        let messages = state.messages.lock().unwrap();
        messages.get("/avatar/parameters/VRMode").and_then(|message| match message.args.first() {
            Some(OscArg::Number(value)) => Some(*value >= 0.5),
            Some(OscArg::Bool(on)) => Some(*on),
            _ => None
        })
    };
    if let Some(vr) = vr.filter(|vr| *vr != settings.last_vr) {
        settings = store::with(app, |settings| {
            settings.last_vr = vr;
            (settings.clone(), true)
        });
    }
    keep_audio(app, &settings, now);

    let paused = crate::automations::is_paused(app);
    let mut run = v.run.lock().unwrap();
    run.holds.expire(now);
    run.cooldowns.retain(|_, until| *until > now);
    if paused && !run.holds.is_empty() {
        run.avatar.stopped = run.holds.is_held(&Source::Avatar);
        run.mouth_stopped = run.holds.is_held(&Source::Mouth);
        run.holds.clear();
    }

    // Holds without their own effect or strength follow the page and the avatar live.
    run.holds.adjust(&Source::You, None, settings.level);
    let (effect, level) = (run.avatar.effect, run.avatar.level.unwrap_or(settings.level));
    run.holds.adjust(&Source::Avatar, effect, level);

    // The mouth socket.
    let mouth = &settings.mouth;
    let sockets = sockets(&run, mouth);
    let inside = Some(followed(&sockets, mouth).map(|socket| socket.depth).fold(0.0, f64::max)).filter(|depth| mouth.enabled && *depth > INSIDE);
    match inside {
        Some(depth) => {
            run.mouth_inside = Some(now);
            let level = mouth.tip + (mouth.deep - mouth.tip) * depth;
            if !paused && !run.mouth_stopped && !run.holds.adjust(&Source::Mouth, Some(mouth.effect), level) {
                run.holds.hold(Source::Mouth, None, Some(mouth.effect), level, None);
            }
        }
        None => {
            run.mouth_stopped = false;
            let waited = run.mouth_inside.map_or(true, |at| now.duration_since(at) >= Duration::from_millis(u64::from(mouth.release_ms)));
            if waited || !mouth.enabled {
                run.holds.release(&Source::Mouth);
            }
        }
    }

    let playing = run.holds.playing().map(|hold| (hold.effect.unwrap_or(settings.default_effect), hold.level, source_kind(hold)));
    v.shared.play(playing.map(|(effect, _, _)| effect), playing.map_or(0.0, |(_, level, _)| level));

    let now_playing = playing.map(|(effect, _, source)| (effect, source));
    let event = match (run.playing, now_playing) {
        (None, Some((_, source))) => Some((VoiceFxEvent::On, source)),
        (Some((_, source)), None) => Some((VoiceFxEvent::Off, source)),
        (Some((before, _)), Some((after, source))) if before != after => Some((VoiceFxEvent::Changed, source)),
        _ => None
    };
    run.playing = now_playing;

    let on = now_playing.is_some();
    let send_on = run.on_sent != Some(on);
    run.on_sent = Some(on);

    let status = status(app, now_playing.map(|(effect, _)| effect)).text;
    let status_changed = run.status != status;
    if status_changed {
        run.status = status;
    }
    let busy = !run.holds.is_empty() || inside.is_some();
    drop(run);

    if send_on {
        send_on_parameter(app, on);
    }
    if status_changed {
        crate::addons::changed(app);
    }
    if let Some((event, source)) = event {
        crate::automations::voicefx_event(app, event, source);
    }
    emit_view(app, false);
    busy
}

fn send_on_parameter(app: &AppHandle, on: bool) {
    let state = app.state::<AppState>();
    let command = OscCommand { address: format!("/avatar/parameters/{ON_PARAMETER}"), args: vec![OscArg::Bool(on)] };
    if let Err(err) = crate::osc::send(&state, &command) {
        tracing::debug!("Couldn't send {ON_PARAMETER}: {err}");
    }
}

fn emit_view(app: &AppHandle, now: bool) {
    let view = view(app);
    let mut sent = voicefx(app).sent.lock().unwrap();
    let due = match &*sent {
        Some((at, last)) => *last != view && (now || at.elapsed() >= VIEW_EVERY),
        None => true
    };
    if due {
        let _ = app.emit("voicefx-state", &view);
        *sent = Some((Instant::now(), view));
    }
}

fn start(app: &AppHandle) {
    let mut task = voicefx(app).task.lock().unwrap();
    if task.is_some() {
        return;
    }
    let details = app.state::<AppState>().avatar_details.lock().unwrap().clone();
    add_sockets(&mut voicefx(app).run.lock().unwrap(), details.as_ref());
    let app = app.clone();
    *task = Some(tauri::async_runtime::spawn(async move {
        loop {
            // Opening a device can take a moment: off the async threads.
            let ticking = app.clone();
            let busy = tauri::async_runtime::spawn_blocking(move || tick(&ticking)).await.unwrap_or(false);
            tokio::select! {
                _ = tokio::time::sleep(if busy { TICK } else { IDLE }) => {}
                _ = voicefx(&app).wake.notified() => {}
            }
        }
    }));
}

fn stop(app: &AppHandle) {
    let v = voicefx(app);
    if let Some(task) = v.task.lock().unwrap().take() {
        task.abort();
    }
    // Closes the streams: VRChat hears nothing through the cable anymore.
    *v.audio.lock().unwrap() = Audio::default();
    v.shared.play(None, 0.0);
    let was_on = {
        let mut run = v.run.lock().unwrap();
        let was_on = run.playing.is_some();
        *run = Run::default();
        was_on
    };
    if was_on {
        send_on_parameter(app, false);
    }
    emit_view(app, true);
}

fn status(app: &AppHandle, playing: Option<Effect>) -> crate::addons::Status {
    use crate::addons::{State, Status};
    let audio = voicefx(app).audio.lock().unwrap();
    if let Some(error) = &audio.error {
        return Status::new(State::Warn, error.trim_end_matches('.'));
    }
    match playing {
        Some(effect) => Status::new(State::Ok, format!("{} is on", effect.label())),
        None if audio.fallback => Status::new(State::Warn, "VR mic not found, using the desktop one"),
        None if audio.running.is_some() => Status::new(State::Ok, "Your voice goes through, clean"),
        None => Status::new(State::Busy, "Starting")
    }
}

/// Voice effects as an add-on (see `crate::addons`).
pub struct Addon;

impl crate::addons::Addon for Addon {
    fn manifest(&self) -> crate::addons::Manifest {
        crate::addons::Manifest {
            id: "voicefx",
            name: "Voice effects",
            group: "voice",
            pitch: "Puts an effect on your voice in VRChat (muffled, underwater, radio), from your avatar, viewers or automations",
            platforms: &["windows", "linux"],
            badges: &["beta"],
            needs: &[],
            needs_any: &[],
            control_types: &["voice-fx"],
            // Its step and its When share a name.
            automation_kinds: &["voice-fx"],
            pages: &["voicefx"]
        }
    }

    fn start(&self, app: &AppHandle) {
        start(app);
    }

    fn stop(&self, app: &AppHandle) {
        stop(app);
    }

    fn status(&self, app: &AppHandle) -> crate::addons::Status {
        let v = voicefx(app);
        if v.task.lock().unwrap().is_none() {
            return crate::addons::Status::new(crate::addons::State::Off, "Off");
        }
        let playing = v.run.lock().unwrap().playing.map(|(effect, _)| effect);
        status(app, playing)
    }

    fn stop_everything(&self, app: &AppHandle) -> crate::addons::StopFuture {
        stop_holds(app);
        Box::pin(async { Ok(()) })
    }

    fn was_in_use(&self, _app: &AppHandle) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AvatarParameter;

    #[test]
    fn sockets_come_from_the_avatar_config() {
        let parameter = |name: &str| AvatarParameter { name: name.into(), input: None, output: None };
        let names = ["OGB/Orf/Blowjob/PenOthersNewTip", "OGB/Orf/Blowjob/PenSelfNewTip", "OGB/Pen/Penis/Others", "VF69_Blowjob"];
        let details = AvatarDetails { id: "avtr_x".into(), name: "x".into(), hash: 0, parameters: names.map(parameter).to_vec() };
        let mut run = Run::default();
        add_sockets(&mut run, Some(&details));
        let found = sockets(&run, &model::Mouth::default());
        assert_eq!(found.iter().map(|socket| socket.name.as_str()).collect::<Vec<_>>(), ["Blowjob"]);
        assert_eq!(found[0].depth, 0.0);
    }

    #[test]
    fn auto_follows_mouths_and_custom_follows_the_ticked() {
        let socket = |name: &str| SocketView { name: name.into(), depth: 0.0 };
        let sockets = [socket("Blowjob"), socket("Mouth 2"), socket("Vagina")];
        let names = |mouth: &model::Mouth| followed(&sockets, mouth).map(|socket| socket.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&model::Mouth::default()), ["Blowjob", "Mouth 2"]);
        let custom = model::Mouth { custom: true, sockets: vec!["Vagina".into(), "Gone".into()], ..Default::default() };
        assert_eq!(names(&custom), ["Vagina"]);
    }
}
