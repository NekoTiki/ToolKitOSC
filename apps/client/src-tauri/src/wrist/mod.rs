//! The wrist panel: `toolkitosc --overlay <port> <token>`, a second process the app starts while
//! the VR overlay add-on is on and SteamVR runs (see `crate::overlay::sidecar`). It's the same
//! binary so there's nothing extra to ship, and a separate process so a crash in SteamVR or here
//! never takes the app down.
//!
//! It only draws and reports presses. The panel rides on one controller, fades in when the back of
//! that hand faces your eyes, and takes a poke from the other hand's fingertip or its trigger
//! while it points at the panel. Every press goes to the app, which acts on it the way its own
//! window would.
mod geometry;
mod input;
mod link;
mod openvr;
pub mod protocol;
mod ui;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use egui::{Pos2, Rect};

use crate::overlay::model::Hand;
use geometry::{look_angle, multiply, position, ray_hit, wrist_offset};
use openvr::{Actions, OpenVr, HMD, INVALID_DEVICE, ROLE_LEFT, ROLE_RIGHT};

/// Pixels per egui point: the panel is drawn at twice its 380 × 270 layout.
const SCALE: f32 = 2.0;
const TICK: Duration = Duration::from_millis(11);
/// Redraws at most this often.
const FRAME: Duration = Duration::from_millis(33);
/// The fade from fully shown to hidden spans this many degrees past `show_within_deg`.
const FADE_DEG: f32 = 15.0;
/// Fully faded for this long before it's hidden, so a glance away doesn't flicker it.
const HIDE_AFTER: Duration = Duration::from_millis(400);
/// Presses count only once it's been fully shown this long.
const SETTLE: Duration = Duration::from_millis(300);
/// A poke ends once the fingertip is back out this far.
const POKE_RELEASE: f32 = 0.015;
/// Closer than this, the tip hovers and pokes; farther, its laser takes over.
const POKE_RANGE: f32 = 0.05;
/// The cursor: a ring this wide, just in front of the panel.
const CURSOR_SIZE: f32 = 0.012;
const CURSOR_LIFT: f32 = 0.002;
const CURSOR_PRESSED: [f32; 3] = [0.65, 0.55, 0.98];
/// SteamVR swaps a new image into an overlay asynchronously, and it blanks meanwhile. So the panel
/// is two overlays in one place: a frame goes to the hidden one, which is shown this long after.
const SWAP_AFTER: Duration = Duration::from_millis(50);
/// The hidden one stays barely visible, so SteamVR keeps its image loaded.
const BACK_ALPHA: f32 = 0.01;

/// Runs the panel until SteamVR quits or the app goes away.
pub fn main(port: u16, token: String) -> Result<(), String> {
    let vr = OpenVr::init()?;
    let link = link::connect(port, token)?;
    let actions = vr.actions(&input::write_manifest()?)?;
    let panels = [
        vr.create_overlay("io.github.nekotiki.toolkitosc.wrist", "ToolKitOSC")?,
        vr.create_overlay("io.github.nekotiki.toolkitosc.wrist.back", "ToolKitOSC")?
    ];
    let front = AtomicUsize::new(0);
    let cursor = vr.create_overlay("io.github.nekotiki.toolkitosc.wrist.cursor", "ToolKitOSC cursor")?;
    vr.set_width(cursor, CURSOR_SIZE);
    vr.set_sort_order(cursor, 2);
    vr.set_raw(cursor, &mut ring(64), 64, 64)?;
    tracing::info!("Panel: on the wrist");
    let (to_render, render_rx) = std::sync::mpsc::channel();
    let pressed = link.actions.clone();
    // Drawing runs beside tracking, so the cursor keeps up however long a frame takes.
    let result = std::thread::scope(|scope| {
        scope.spawn(|| render(&vr, panels, &front, render_rx, pressed));
        run(&vr, &link, &actions, panels, &front, cursor, to_render)
    });
    vr.destroy_overlay(cursor);
    panels.iter().for_each(|&panel| vr.destroy_overlay(panel));
    result
}

/// A white ring with a dark edge, `size` pixels square, as RGBA.
fn ring(size: usize) -> Vec<u8> {
    let mut rgba = vec![0u8; size * size * 4];
    let center = size as f32 / 2.0;
    for y in 0..size {
        for x in 0..size {
            let d = ((x as f32 + 0.5 - center).powi(2) + (y as f32 + 0.5 - center).powi(2)).sqrt() / center;
            let (value, alpha) = match d {
                d if d < 0.6 => (255, 70),
                d if d < 0.8 => (255, 255),
                d if d < 0.95 => (20, 200),
                _ => (0, 0)
            };
            rgba[(y * size + x) * 4..][..4].copy_from_slice(&[value, value, value, alpha]);
        }
    }
    rgba
}

#[derive(PartialEq)]
enum Pointer {
    None,
    /// Panel-space x and y in meters, and whether it's pressing.
    At([f32; 2], bool)
}

enum ToRender {
    State(protocol::State),
    Events(Vec<egui::Event>),
    /// Shown or hidden: nothing is drawn while it's hidden.
    Visible(bool)
}

/// Draws the panel while it's shown: whenever it gets new state or input, or a hold is filling,
/// and once a second for "8 s ago". What comes in while it's hidden is drawn when it shows again.
/// Stops when tracking does.
fn render(vr: &OpenVr, panels: [u64; 2], front: &AtomicUsize, rx: std::sync::mpsc::Receiver<ToRender>, pressed: std::sync::mpsc::Sender<protocol::ToApp>) {
    use std::sync::mpsc::RecvTimeoutError;

    let ctx = egui::Context::default();
    ctx.style_mut(|style| style.animation_time = 0.0);
    // Text is laid out for the pixels it's drawn at.
    ctx.set_pixels_per_point(SCALE);
    let mut panel = ui::Panel::default();
    let mut raster = toolkitosc_raster::Raster::default();
    let size = ui::SIZE * SCALE;
    let (width, height) = (size.x as usize, size.y as usize);
    let mut pixels = vec![[0.0f32; 4]; width * height];
    let mut bytes = Vec::with_capacity(width * height * 4);
    let mut sent: Vec<u8> = Vec::new();
    let started = Instant::now();
    let mut events: Vec<egui::Event> = Vec::new();
    let mut again = false;
    let mut retry = false;
    let mut visible = false;
    let mut last_frame = Instant::now() - FRAME;

    loop {
        let wait = if !visible {
            // Nothing to draw until something comes in.
            None
        } else if again || retry {
            Some(FRAME)
        } else {
            Some(Duration::from_secs(1))
        };
        let mut apply = |message: ToRender| match message {
            ToRender::State(state) => panel.set_state(state),
            ToRender::Events(more) => events.extend(more),
            ToRender::Visible(shown) => visible = shown
        };
        match wait.map_or_else(|| rx.recv().map_err(|_| RecvTimeoutError::Disconnected), |wait| rx.recv_timeout(wait)) {
            Ok(message) => apply(message),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return
        }
        // At most one frame every `FRAME`: what comes in meanwhile goes into the next one.
        std::thread::sleep(FRAME.saturating_sub(last_frame.elapsed()));
        while let Ok(message) = rx.try_recv() {
            apply(message);
        }
        if !visible {
            continue;
        }
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, ui::SIZE)),
            time: Some(started.elapsed().as_secs_f64()),
            events: std::mem::take(&mut events),
            ..Default::default()
        };
        let mut actions = Vec::new();
        let output = ctx.run(input, |ctx| (actions, again) = panel.show(ctx));
        for action in actions {
            let _ = pressed.send(action);
        }
        raster.update_textures(&output.textures_delta);
        let primitives = ctx.tessellate(output.shapes, SCALE);
        raster.draw(&primitives, SCALE, &mut pixels, width, height);
        toolkitosc_raster::to_rgba8(&pixels, &mut bytes);
        // Every upload can flicker in SteamVR, so only a frame that looks different goes out.
        if bytes != sent {
            let back = 1 - front.load(Ordering::Relaxed);
            match vr.set_raw(panels[back], &mut bytes, width as u32, height as u32) {
                Ok(()) => {
                    std::thread::sleep(SWAP_AFTER);
                    front.store(back, Ordering::Relaxed);
                    std::mem::swap(&mut bytes, &mut sent);
                    retry = false;
                }
                // SteamVR refuses an upload while it's still taking the last one: the panel keeps
                // what it shows, and the same frame goes again next time.
                Err(err) => {
                    tracing::debug!("Panel: {err}, trying again");
                    retry = true;
                }
            }
        }
        last_frame = Instant::now();
    }
}

/// Tracks the hands every tick: shows and hides the panel, moves the cursor, and turns pokes and
/// trigger pulls into presses for `render`.
fn run(vr: &OpenVr, link: &link::Link, actions: &Actions, panels: [u64; 2], front: &AtomicUsize, cursor: u64, to_render: std::sync::mpsc::Sender<ToRender>) -> Result<(), String> {
    let mut settings = crate::overlay::model::PanelSettings::default();
    let mut attached: Option<(u32, Hand)> = None;
    let mut width_set = 0.0f32;
    let mut alpha = -1.0f32;
    let mut alpha_front = usize::MAX;
    let mut visible = false;
    let mut faded_since: Option<Instant> = None;
    let mut shown_since: Option<Instant> = None;
    let mut pointer = Pointer::None;
    // The fingertip's side of the panel last tick, for a poke going in.
    let mut tip_in_front = true;
    let mut poking = false;
    let mut events: Vec<egui::Event> = Vec::new();
    let mut cursor_shown = false;

    // What `render` was last told.
    let mut told_visible = false;

    loop {
        std::thread::sleep(TICK);
        if visible != told_visible {
            told_visible = visible;
            let _ = to_render.send(ToRender::Visible(visible));
        }
        if vr.quit_requested() {
            tracing::info!("Panel: SteamVR is quitting");
            return Ok(());
        }
        loop {
            match link.states.try_recv() {
                Ok(state) => {
                    settings = state.panel.clone();
                    let _ = to_render.send(ToRender::State(state));
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return Ok(())
            }
        }
        if !settings.enabled {
            if visible {
                panels.iter().for_each(|&panel| vr.set_visible(panel, false));
                visible = false;
            }
            continue;
        }
        let width_m = settings.width_cm.clamp(10.0, 30.0) / 100.0;
        let height_m = width_m * ui::SIZE.y / ui::SIZE.x;

        let left = settings.hand == Hand::Left;
        let role = if left { ROLE_LEFT } else { ROLE_RIGHT };
        let device = vr.controller(role);
        let poses = vr.poses();
        let pose = |index: u32| poses.get(index as usize).filter(|pose| index != INVALID_DEVICE && pose.valid);
        let (Some(hand_pose), Some(hmd)) = (pose(device), pose(HMD)) else {
            if visible {
                panels.iter().for_each(|&panel| vr.set_visible(panel, false));
                visible = false;
            }
            continue;
        };

        if attached != Some((device, settings.hand)) {
            panels.iter().for_each(|&panel| vr.attach(panel, device, &wrist_offset(left)));
            attached = Some((device, settings.hand));
        }
        if width_set != width_m {
            panels.iter().for_each(|&panel| vr.set_width(panel, width_m));
            width_set = width_m;
        }

        // Look to show.
        let panel_world = multiply(&hand_pose.device_to_absolute, &wrist_offset(settings.hand == Hand::Left));
        let angle = look_angle(&panel_world, position(&hmd.device_to_absolute));
        let show = settings.show_within_deg.clamp(10.0, 80.0);
        let fade = (1.0 - (angle - show) / FADE_DEG).clamp(0.0, 1.0);
        let opacity = settings.opacity.clamp(0.3, 1.0);
        let now = Instant::now();
        if fade > 0.0 {
            faded_since = None;
            if !visible {
                panels.iter().for_each(|&panel| vr.set_visible(panel, true));
                visible = true;
            }
        } else if visible && now.duration_since(*faded_since.get_or_insert(now)) >= HIDE_AFTER {
            panels.iter().for_each(|&panel| vr.set_visible(panel, false));
            visible = false;
        }
        let shown = front.load(Ordering::Relaxed);
        if (alpha - fade * opacity).abs() > 0.01 || shown != alpha_front {
            alpha = fade * opacity;
            alpha_front = shown;
            vr.set_alpha(panels[shown], alpha);
            vr.set_sort_order(panels[shown], 1);
            vr.set_alpha(panels[1 - shown], alpha.min(BACK_ALPHA));
            vr.set_sort_order(panels[1 - shown], 0);
        }
        if fade >= 1.0 {
            shown_since.get_or_insert(now);
        } else {
            shown_since = None;
        }
        let settled = shown_since.is_some_and(|since| now.duration_since(since) >= SETTLE);

        // The other hand: a poke with its tip up close, its laser from farther away.
        vr.update_actions(actions);
        let other = vr.pointer(actions, !left);
        let click = vr.click(actions, !left);
        let inside = |p: [f32; 2]| p[0].abs() <= width_m / 2.0 && p[1].abs() <= height_m / 2.0;
        let next = match other {
            Some(other) if visible => {
                let tip = geometry::into(&panel_world, position(&other));
                let near = tip[2] < POKE_RANGE && tip[2] > -0.03 && inside([tip[0], tip[1]]);
                if near {
                    if tip_in_front && tip[2] <= 0.0 && settled {
                        poking = true;
                    }
                    if poking && tip[2] > POKE_RELEASE {
                        poking = false;
                    }
                    tip_in_front = tip[2] > 0.0;
                    Pointer::At([tip[0], tip[1]], poking)
                } else {
                    tip_in_front = tip[2] > 0.0 || !inside([tip[0], tip[1]]);
                    poking = false;
                    match ray_hit(&panel_world, position(&other), geometry::direction(&other, [0.0, 0.0, -1.0])) {
                        Some((hit, _)) if inside(hit) => Pointer::At(hit, settled && click),
                        _ => Pointer::None
                    }
                }
            }
            _ => {
                poking = false;
                tip_in_front = true;
                Pointer::None
            }
        };
        // The cursor follows every move without redrawing the panel.
        match next {
            Pointer::At([x, y], down) => {
                vr.attach(cursor, device, &geometry::offset(&wrist_offset(left), x, y, CURSOR_LIFT));
                if !cursor_shown || matches!(pointer, Pointer::At(_, was) if was != down) {
                    vr.set_color(cursor, if down { CURSOR_PRESSED } else { [1.0; 3] });
                }
                if !cursor_shown {
                    vr.set_visible(cursor, true);
                    cursor_shown = true;
                }
            }
            Pointer::None if cursor_shown => {
                vr.set_visible(cursor, false);
                cursor_shown = false;
            }
            Pointer::None => {}
        }
        if next != pointer {
            let to_points = |p: [f32; 2]| Pos2::new((p[0] / width_m + 0.5) * ui::SIZE.x, (0.5 - p[1] / height_m) * ui::SIZE.y);
            let was_down = matches!(pointer, Pointer::At(_, true));
            match next {
                Pointer::At(p, down) => {
                    let pos = to_points(p);
                    events.push(egui::Event::PointerMoved(pos));
                    if down != was_down {
                        events.push(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: down, modifiers: Default::default() });
                    }
                }
                Pointer::None => {
                    if was_down {
                        events.push(egui::Event::PointerButton { pos: Pos2::ZERO, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
                    }
                    events.push(egui::Event::PointerGone);
                }
            }
            pointer = next;
        }
        if !events.is_empty() {
            let _ = to_render.send(ToRender::Events(std::mem::take(&mut events)));
        }
    }
}

