//! What the panel shows, in egui, at 380 × 270 points: the rail on the left (Home, Viewers,
//! Pinned, Add-ons, and Stop at its bottom on every page), the page on the right. Drawn with the
//! app's dark Aurora colors. Everything is big: it's poked with a controller tip or hit with a
//! laser, and nothing scrolls.
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use egui::{Align2, Color32, FontId, Pos2, Rect, Rounding, Stroke, Vec2};

use super::protocol::{Entry, State, Tile, TileKind, ToApp};

pub const SIZE: Vec2 = Vec2::new(380.0, 270.0);
const RAIL: f32 = 58.0;
const RESUME_HOLD: Duration = Duration::from_millis(1500);
/// Shocks and the OWO Skin fire after this long a hold.
const FIRE_HOLD: Duration = Duration::from_millis(600);
/// A dragged level goes out once it's moved this much.
const LEVEL_STEP: f64 = 0.02;

const BG: Color32 = Color32::from_rgba_premultiplied(17, 20, 31, 235);
const WELL: Color32 = Color32::from_rgba_premultiplied(30, 34, 46, 255);
const LINE: Color32 = Color32::from_rgba_premultiplied(40, 44, 56, 255);
const FG: Color32 = Color32::from_rgb(0xee, 0xf0, 0xf7);
const MUTED: Color32 = Color32::from_rgb(0x9a, 0xa0, 0xb4);
const PRIMARY: Color32 = Color32::from_rgb(0xa7, 0x8b, 0xfa);
const SECONDARY: Color32 = Color32::from_rgb(0x22, 0xd3, 0xee);
const HOT: Color32 = Color32::from_rgb(0xfb, 0xbf, 0x24);
const OK: Color32 = Color32::from_rgb(0x4a, 0xde, 0x80);
const STOP: Color32 = Color32::from_rgb(0xf4, 0x3f, 0x5e);

#[derive(Clone, Copy, Default, PartialEq)]
enum Page {
    #[default]
    Home,
    Viewers,
    Pinned,
    Addons
}

#[derive(Default)]
pub struct Panel {
    pub state: State,
    page: Page,
    picking_profile: bool,
    /// Viewers shows Activity instead of who's online.
    activity: bool,
    /// What's being held, since when, and whether it already fired.
    hold: Option<(String, Instant, bool)>,
    /// The level last sent for the tile being dragged.
    dragging: Option<(String, f64)>
}

/// Pressed this frame on `rect`: a poke going in or a trigger pull. Fires on the way in, not on
/// release, so Stop is one poke.
fn pressed(ui: &egui::Ui, rect: Rect) -> bool {
    ui.input(|i| i.pointer.primary_pressed() && i.pointer.interact_pos().is_some_and(|p| rect.contains(p)))
}

/// Where the pointer holds `rect` down, if it does.
fn held(ui: &egui::Ui, rect: Rect) -> Option<Pos2> {
    ui.input(|i| i.pointer.interact_pos().filter(|p| i.pointer.primary_down() && rect.contains(*p)))
}

fn hovered(ui: &egui::Ui, rect: Rect) -> bool {
    ui.input(|i| i.pointer.hover_pos().is_some_and(|p| rect.contains(p)))
}

fn text(ui: &egui::Ui, pos: Pos2, anchor: Align2, text: &str, size: f32, color: Color32) -> Rect {
    ui.painter().text(pos, anchor, text, FontId::proportional(size), color)
}

/// Cut to fit `width` points, with an ellipsis.
fn fit(ui: &egui::Ui, text: &str, size: f32, width: f32) -> String {
    let fits = |t: &str| ui.painter().layout_no_wrap(t.to_owned(), FontId::proportional(size), FG).size().x <= width;
    if fits(text) {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() && !fits(&format!("{}…", chars.iter().collect::<String>())) {
        chars.pop();
    }
    format!("{}…", chars.into_iter().collect::<String>())
}

/// A row with a title, a muted line under it, and room for something on its right.
fn row(ui: &egui::Ui, rect: Rect, title: &str, sub: &str, right: f32) {
    ui.painter().rect_filled(rect, Rounding::same(12.0), WELL);
    let width = rect.width() - 20.0 - right;
    text(ui, rect.left_top() + Vec2::new(10.0, 7.0), Align2::LEFT_TOP, &fit(ui, title, 13.0, width), 13.0, FG);
    text(ui, rect.left_top() + Vec2::new(10.0, 24.0), Align2::LEFT_TOP, &fit(ui, sub, 11.0, width), 11.0, MUTED);
}

fn button(ui: &egui::Ui, rect: Rect, label: &str, fill: Color32, color: Color32) -> bool {
    let fill = if hovered(ui, rect) { fill.gamma_multiply(1.25) } else { fill };
    ui.painter().rect_filled(rect, Rounding::same(255.0), fill);
    text(ui, rect.center(), Align2::CENTER_CENTER, label, 12.5, color);
    pressed(ui, rect)
}

/// A pill of two choices; returns the one pressed.
fn segmented(ui: &egui::Ui, rect: Rect, labels: [&str; 2], on: usize) -> Option<usize> {
    ui.painter().rect_filled(rect, Rounding::same(255.0), WELL);
    let half = rect.width() / 2.0;
    let mut picked = None;
    for (i, label) in labels.iter().enumerate() {
        let part = Rect::from_min_size(rect.min + Vec2::new(i as f32 * half, 0.0), Vec2::new(half, rect.height())).shrink(3.0);
        if i == on {
            ui.painter().rect_filled(part, Rounding::same(255.0), PRIMARY.gamma_multiply(0.35));
        }
        text(ui, part.center(), Align2::CENTER_CENTER, label, 11.5, if i == on { FG } else { MUTED });
        if pressed(ui, part) {
            picked = Some(i);
        }
    }
    picked
}

/// A thin bar filled to `level`, 0–1.
fn bar(ui: &egui::Ui, rect: Rect, level: f64, color: Color32) {
    ui.painter().rect_filled(rect, Rounding::same(255.0), LINE);
    let mut fill = rect;
    fill.set_width(rect.width() * level.clamp(0.0, 1.0) as f32);
    ui.painter().rect_filled(fill, Rounding::same(255.0), color);
}

fn ago(at: u64) -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
    let seconds = now.saturating_sub(at) / 1000;
    match seconds {
        0..=59 => format!("{seconds} s"),
        60..=3599 => format!("{} min", seconds / 60),
        _ => format!("{} h", seconds / 3600)
    }
}

fn entry_text(entry: &Entry) -> String {
    format!("{} {}", entry.who, entry.what)
}

impl Panel {
    /// Draws the panel. Returns what was pressed, and whether it needs to draw again soon (a hold
    /// is filling).
    pub fn show(&mut self, ctx: &egui::Context) -> (Vec<ToApp>, bool) {
        let mut actions = Vec::new();
        let mut again = false;
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            let full = Rect::from_min_size(Pos2::ZERO, SIZE);
            ui.painter().rect_filled(full, Rounding::same(22.0), BG);
            ui.painter().rect_stroke(full.shrink(0.5), Rounding::same(22.0), Stroke::new(1.0_f32, LINE));
            self.rail(ui, &mut actions);
            let main = Rect::from_min_max(Pos2::new(RAIL + 14.0, 12.0), Pos2::new(SIZE.x - 14.0, SIZE.y - 12.0));
            again = match self.page {
                Page::Home if self.picking_profile => self.profiles(ui, main, &mut actions),
                Page::Home => self.home(ui, main, &mut actions),
                Page::Viewers => self.viewers(ui, main),
                Page::Pinned => self.pinned(ui, main, &mut actions),
                Page::Addons => self.addons(ui, main)
            };
            // Nothing held any more: a hold or a drag ends.
            if !ui.input(|i| i.pointer.primary_down()) {
                self.hold = None;
                self.dragging = None;
            }
        });
        (actions, again)
    }

    /// Fills a hold on `key` while `down`. Returns how far it is, 0–1, and true the moment it
    /// completes.
    fn hold(&mut self, key: &str, down: bool, length: Duration) -> (f32, bool) {
        if !down {
            if self.hold.as_ref().is_some_and(|(held, _, _)| held == key) {
                self.hold = None;
            }
            return (0.0, false);
        }
        let (_, start, fired) = match &mut self.hold {
            Some(hold) if hold.0 == key => hold,
            hold => hold.insert((key.to_owned(), Instant::now(), false))
        };
        let progress = (start.elapsed().as_secs_f32() / length.as_secs_f32()).min(1.0);
        let now = progress >= 1.0 && !*fired;
        *fired |= now;
        (progress, now)
    }

    fn rail(&mut self, ui: &egui::Ui, actions: &mut Vec<ToApp>) {
        ui.painter().line_segment([Pos2::new(RAIL, 0.0), Pos2::new(RAIL, SIZE.y)], Stroke::new(1.0_f32, LINE));
        let pages = [(Page::Home, "🏠"), (Page::Viewers, "👥"), (Page::Pinned, "📌"), (Page::Addons, "🔌")];
        for (i, (page, icon)) in pages.into_iter().enumerate() {
            let rect = Rect::from_min_size(Pos2::new(6.0, 8.0 + i as f32 * 46.0), Vec2::new(RAIL - 12.0, 42.0));
            if page == self.page {
                ui.painter().rect_filled(rect, Rounding::same(14.0), PRIMARY.gamma_multiply(0.28));
                ui.painter().rect_stroke(rect, Rounding::same(14.0), Stroke::new(1.5_f32, PRIMARY));
            } else if hovered(ui, rect) {
                ui.painter().rect_filled(rect, Rounding::same(14.0), WELL);
            }
            text(ui, rect.center(), Align2::CENTER_CENTER, icon, 20.0, if page == self.page { FG } else { MUTED });
            if pressed(ui, rect) {
                self.page = page;
                self.picking_profile = false;
            }
        }

        // Below the last page with a clear gap, so a poke can't land on both.
        let stop = Rect::from_min_size(Pos2::new(6.0, SIZE.y - 8.0 - 56.0), Vec2::new(RAIL - 12.0, 56.0));
        let fill = if hovered(ui, stop) { STOP.gamma_multiply(1.2) } else { STOP };
        ui.painter().rect_filled(stop, Rounding::same(16.0), fill);
        text(ui, stop.center(), Align2::CENTER_CENTER, "STOP", 11.0, Color32::WHITE);
        if pressed(ui, stop) {
            actions.push(ToApp::Stop);
        }
    }

    /// The page's title on the left, and a chip on the right.
    fn header(ui: &egui::Ui, main: Rect, title: &str, chip: Option<(&str, Color32)>) {
        text(ui, main.left_top() + Vec2::new(0.0, 4.0), Align2::LEFT_TOP, title, 15.0, FG);
        let Some((chip, color)) = chip else {
            return;
        };
        let galley = ui.painter().layout_no_wrap(chip.into(), FontId::proportional(11.5), color);
        let rect = Rect::from_min_size(Pos2::new(main.right() - galley.size().x - 18.0, main.top() + 2.0), galley.size() + Vec2::new(18.0, 6.0));
        ui.painter().rect_filled(rect, Rounding::same(255.0), color.gamma_multiply(0.18));
        ui.painter().galley(rect.min + Vec2::new(9.0, 3.0), galley, color);
    }

    fn home(&mut self, ui: &egui::Ui, main: Rect, actions: &mut Vec<ToApp>) -> bool {
        let paused = self.state.paused;
        Self::header(ui, main, "ToolKitOSC", Some(if paused { ("Paused", STOP) } else { ("Sharing", OK) }));

        let mut y = main.top() + 30.0;
        let mut again = false;
        if paused {
            // Resume needs a hold, so a stray poke can't undo a Stop.
            let rect = Rect::from_min_size(Pos2::new(main.left(), y), Vec2::new(main.width(), 62.0));
            let (progress, done) = self.hold("resume", held(ui, rect).is_some(), RESUME_HOLD);
            if done {
                actions.push(ToApp::Resume);
            }
            again = progress > 0.0 && progress < 1.0;
            ui.painter().rect_filled(rect, Rounding::same(14.0), WELL);
            let mut fill = rect;
            fill.set_width(rect.width() * progress);
            ui.painter().rect_filled(fill, Rounding::same(14.0), OK.gamma_multiply(0.45));
            text(ui, rect.center() - Vec2::new(0.0, 8.0), Align2::CENTER_CENTER, "Hold to resume", 15.0, FG);
            text(ui, rect.center() + Vec2::new(0.0, 12.0), Align2::CENTER_CENTER, "Viewers can't use your controls", 11.0, MUTED);
        } else {
            let state = &self.state;
            let half = (main.width() - 8.0) / 2.0;
            let stats = [
                (state.watching.to_string(), format!("watching · {} signed in", state.signed_in)),
                (state.recent.to_string(), "actions in the last minute".to_string())
            ];
            for (i, (big, small)) in stats.iter().enumerate() {
                let rect = Rect::from_min_size(Pos2::new(main.left() + i as f32 * (half + 8.0), y), Vec2::new(half, 62.0));
                ui.painter().rect_filled(rect, Rounding::same(14.0), WELL);
                text(ui, rect.left_top() + Vec2::new(12.0, 8.0), Align2::LEFT_TOP, big, 22.0, FG);
                text(ui, rect.left_top() + Vec2::new(12.0, 40.0), Align2::LEFT_TOP, &fit(ui, small, 11.0, half - 18.0), 11.0, MUTED);
            }
        }
        y += 70.0;

        let state = &self.state;
        let profile = state.profile.as_ref().and_then(|id| state.profiles.iter().find(|p| &p.id == id)).map_or("No profile", |p| p.name.as_str());
        let rect = Rect::from_min_size(Pos2::new(main.left(), y), Vec2::new(main.width(), 46.0));
        row(ui, rect, "Profile", profile, 90.0);
        let change = Rect::from_min_size(Pos2::new(rect.right() - 86.0, rect.top() + 8.0), Vec2::new(78.0, 30.0));
        if !state.profiles.is_empty() && button(ui, change, "Change", WELL.gamma_multiply(1.6), FG) {
            self.picking_profile = true;
        }
        y += 54.0;

        if let Some(last) = self.state.activity.first() {
            let rect = Rect::from_min_size(Pos2::new(main.left(), y), Vec2::new(main.width(), 46.0));
            row(ui, rect, "Last", &entry_text(last), 50.0);
            text(ui, Pos2::new(rect.right() - 12.0, rect.center().y), Align2::RIGHT_CENTER, &ago(last.at), 11.5, MUTED);
        }
        again
    }

    fn profiles(&mut self, ui: &egui::Ui, main: Rect, actions: &mut Vec<ToApp>) -> bool {
        Self::header(ui, main, "Profile", None);
        let mut y = main.top() + 30.0;
        let choices = std::iter::once((None, "No profile")).chain(self.state.profiles.iter().map(|p| (Some(p.id.clone()), p.name.as_str())));
        let mut picked = None;
        for (id, name) in choices {
            let rect = Rect::from_min_size(Pos2::new(main.left(), y), Vec2::new(main.width(), 36.0));
            if rect.bottom() > main.bottom() {
                break;
            }
            let on = id == self.state.profile;
            let fill = if on { PRIMARY.gamma_multiply(0.3) } else if hovered(ui, rect) { WELL.gamma_multiply(1.4) } else { WELL };
            ui.painter().rect_filled(rect, Rounding::same(12.0), fill);
            text(ui, rect.left_center() + Vec2::new(12.0, 0.0), Align2::LEFT_CENTER, &fit(ui, name, 13.0, rect.width() - 24.0), 13.0, FG);
            if pressed(ui, rect) {
                picked = Some(id);
            }
            y += 42.0;
        }
        if let Some(id) = picked {
            actions.push(ToApp::Profile { id });
            self.picking_profile = false;
        }
        false
    }

    fn viewers(&mut self, ui: &egui::Ui, main: Rect) -> bool {
        Self::header(ui, main, "Viewers", None);
        let toggle = Rect::from_min_size(Pos2::new(main.right() - 150.0, main.top()), Vec2::new(150.0, 26.0));
        if let Some(picked) = segmented(ui, toggle, ["Online", "Activity"], usize::from(self.activity)) {
            self.activity = picked == 1;
        }
        let mut y = main.top() + 34.0;
        let space = |y: f32, height: f32| y + height <= main.bottom();
        if self.activity {
            if self.state.activity.is_empty() {
                text(ui, Pos2::new(main.left(), y + 6.0), Align2::LEFT_TOP, "Nothing yet", 13.0, MUTED);
            }
            for entry in &self.state.activity {
                if !space(y, 34.0) {
                    break;
                }
                let rect = Rect::from_min_size(Pos2::new(main.left(), y), Vec2::new(main.width(), 34.0));
                ui.painter().rect_filled(rect, Rounding::same(10.0), WELL);
                text(ui, rect.left_center() + Vec2::new(10.0, 0.0), Align2::LEFT_CENTER, &fit(ui, &entry_text(entry), 12.0, rect.width() - 64.0), 12.0, FG);
                text(ui, rect.right_center() - Vec2::new(10.0, 0.0), Align2::RIGHT_CENTER, &ago(entry.at), 11.0, MUTED);
                y += 38.0;
            }
            return false;
        }
        let viewers = &self.state.viewers;
        if viewers.is_empty() {
            text(ui, Pos2::new(main.left(), y + 6.0), Align2::LEFT_TOP, "Nobody on your share page", 13.0, MUTED);
        }
        for (i, viewer) in viewers.iter().enumerate() {
            let rect = Rect::from_min_size(Pos2::new(main.left(), y), Vec2::new(main.width(), 44.0));
            // The last row says how many more there are.
            if !space(y, 44.0 + 22.0) && i + 1 < viewers.len() {
                text(ui, Pos2::new(main.left() + 4.0, y + 4.0), Align2::LEFT_TOP, &format!("+{} more", viewers.len() - i), 12.0, MUTED);
                break;
            }
            let groups = if viewer.groups.is_empty() { if viewer.signed_in { "Signed in".to_owned() } else { "Guest".to_owned() } } else { viewer.groups.join(", ") };
            let sub = match &viewer.last {
                Some(last) => format!("{groups} · {}", last.what),
                None => groups
            };
            row(ui, rect, &viewer.name, &sub, 44.0);
            ui.painter().circle_filled(Pos2::new(rect.right() - 16.0, rect.top() + 14.0), 4.0, OK);
            if let Some(last) = &viewer.last {
                text(ui, Pos2::new(rect.right() - 10.0, rect.bottom() - 9.0), Align2::RIGHT_BOTTOM, &ago(last.at), 10.5, MUTED);
            }
            y += 48.0;
        }
        false
    }

    fn pinned(&mut self, ui: &egui::Ui, main: Rect, actions: &mut Vec<ToApp>) -> bool {
        let avatar = self.state.avatar.clone().unwrap_or_else(|| "No avatar".into());
        Self::header(ui, main, "Pinned", Some((&fit(ui, &avatar, 11.5, 130.0), MUTED)));
        let top = main.top() + 30.0;
        if self.state.pinned.is_empty() {
            text(ui, Pos2::new(main.left(), top + 6.0), Align2::LEFT_TOP, "This avatar has no controls yet", 13.0, MUTED);
            return false;
        }
        let hint = if self.state.nothing_pinned { 18.0 } else { 0.0 };
        let (cols, rows) = (2, 3);
        let gap = 8.0;
        let width = (main.width() - gap) / cols as f32;
        let height = ((main.bottom() - top - hint) - gap * (rows - 1) as f32) / rows as f32;
        let mut again = false;
        for (i, tile) in self.state.pinned.clone().iter().enumerate().take(cols * rows) {
            let at = Pos2::new(main.left() + (i % cols) as f32 * (width + gap), top + (i / cols) as f32 * (height + gap));
            again |= self.tile(ui, Rect::from_min_size(at, Vec2::new(width, height)), tile, actions);
        }
        if self.state.nothing_pinned {
            text(ui, Pos2::new(main.left(), main.bottom()), Align2::LEFT_BOTTOM, "Pin up to 6 with Pin to wrist in the control editor", 10.5, MUTED);
        }
        again
    }

    fn tile(&mut self, ui: &egui::Ui, rect: Rect, tile: &Tile, actions: &mut Vec<ToApp>) -> bool {
        let key = format!("{}/{}", tile.group_id, tile.control_id);
        let usable = !tile.locked && tile.kind != TileKind::AppOnly;
        let on = matches!(tile.kind, TileKind::Toggle { on: true });
        let (fill, border) = match &tile.kind {
            _ if !usable => (WELL.gamma_multiply(0.6), LINE),
            TileKind::Toggle { on: true } => (PRIMARY.gamma_multiply(0.2), PRIMARY.gamma_multiply(0.7)),
            TileKind::Hold => (WELL, STOP.gamma_multiply(0.6)),
            _ if hovered(ui, rect) => (WELL.gamma_multiply(1.3), LINE),
            _ => (WELL, LINE)
        };
        ui.painter().rect_filled(rect, Rounding::same(14.0), fill);
        ui.painter().rect_stroke(rect, Rounding::same(14.0), Stroke::new(1.0_f32, border));
        let color = if usable { FG } else { MUTED };
        text(ui, rect.left_top() + Vec2::new(10.0, 8.0), Align2::LEFT_TOP, &fit(ui, &tile.name, 12.5, rect.width() - 56.0), 12.5, color);
        let detail_at = rect.left_bottom() + Vec2::new(10.0, -9.0);
        let detail = |label: &str| text(ui, detail_at, Align2::LEFT_BOTTOM, &fit(ui, label, 11.0, rect.width() - 20.0), 11.0, MUTED);
        let down = if usable { held(ui, rect) } else { None };
        let press = || ToApp::Control { group_id: tile.group_id.clone(), control_id: tile.control_id.clone(), level: None };
        let mut again = false;
        match &tile.kind {
            _ if tile.locked => {
                detail("🔒 Locked");
            }
            TileKind::AppOnly => {
                detail("In the app");
            }
            TileKind::Toggle { .. } => {
                detail(if on { "On" } else { "Off" });
                if pressed(ui, rect) {
                    actions.push(press());
                }
            }
            TileKind::Choice { option } => {
                detail(option.as_deref().unwrap_or("Press for the next"));
                if pressed(ui, rect) {
                    actions.push(press());
                }
            }
            TileKind::Press => {
                detail("Press to apply");
                if pressed(ui, rect) {
                    actions.push(press());
                }
            }
            TileKind::Level { value } => {
                // Dragged along the tile; the level shows right away, before the app echoes it.
                let level = match down {
                    Some(pos) => {
                        let level = f64::from(((pos.x - rect.left() - 10.0) / (rect.width() - 20.0)).clamp(0.0, 1.0));
                        let sent = self.dragging.as_ref().filter(|(dragged, _)| *dragged == key).map(|(_, sent)| *sent);
                        // The ends always go out, so a drag can reach 0 and 100 %.
                        let moved = sent.map_or(true, |sent| (sent - level).abs() >= LEVEL_STEP || ((level == 0.0 || level == 1.0) && sent != level));
                        if moved {
                            self.dragging = Some((key.clone(), level));
                            actions.push(ToApp::Control { group_id: tile.group_id.clone(), control_id: tile.control_id.clone(), level: Some(level) });
                        }
                        level
                    }
                    None => *value
                };
                bar(ui, Rect::from_min_size(rect.left_bottom() + Vec2::new(10.0, -22.0), Vec2::new(rect.width() - 20.0, 8.0)), level, SECONDARY);
                text(ui, rect.right_top() + Vec2::new(-10.0, 8.0), Align2::RIGHT_TOP, &format!("{} %", (level * 100.0).round()), 11.0, MUTED);
            }
            TileKind::Hold => {
                let (progress, fire) = self.hold(&key, down.is_some(), FIRE_HOLD);
                if fire {
                    actions.push(press());
                }
                again = progress > 0.0 && progress < 1.0;
                bar(ui, Rect::from_min_size(rect.left_bottom() + Vec2::new(10.0, -22.0), Vec2::new(rect.width() - 20.0, 6.0)), f64::from(progress), STOP);
                detail(if progress >= 1.0 { "Sent" } else { "Hold to fire" });
            }
        }
        again
    }

    fn addons(&mut self, ui: &egui::Ui, main: Rect) -> bool {
        Self::header(ui, main, "Add-ons", None);
        let mut y = main.top() + 30.0;
        if self.state.addons.is_empty() {
            text(ui, Pos2::new(main.left(), y + 6.0), Align2::LEFT_TOP, "No add-ons are on", 13.0, MUTED);
        }
        for addon in &self.state.addons {
            if y + 44.0 > main.bottom() {
                break;
            }
            let rect = Rect::from_min_size(Pos2::new(main.left(), y), Vec2::new(main.width(), 44.0));
            let right = if addon.level.is_some() { 84.0 } else if addon.touched { 64.0 } else { 0.0 };
            row(ui, rect, &addon.name, &addon.status, right);
            if let Some(level) = addon.level {
                let color = if addon.touched { HOT } else { SECONDARY };
                bar(ui, Rect::from_min_size(Pos2::new(rect.right() - 78.0, rect.bottom() - 14.0), Vec2::new(66.0, 6.0)), level, color);
            }
            if addon.touched {
                text(ui, Pos2::new(rect.right() - 12.0, rect.top() + 8.0), Align2::RIGHT_TOP, "Touched", 11.0, HOT);
            }
            y += 48.0;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)), events, ..Default::default() }
    }

    fn button_event(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() }
    }

    fn press_at(ctx: &egui::Context, panel: &mut Panel, pos: Pos2) -> Vec<ToApp> {
        let _ = ctx.run(input(vec![egui::Event::PointerMoved(pos)]), |ctx| {
            panel.show(ctx);
        });
        let mut actions = Vec::new();
        let _ = ctx.run(input(vec![button_event(pos, true)]), |ctx| actions = panel.show(ctx).0);
        let _ = ctx.run(input(vec![button_event(pos, false)]), |ctx| {
            panel.show(ctx);
        });
        actions
    }

    #[test]
    fn stop_fires_on_the_first_poke() {
        let ctx = egui::Context::default();
        let mut panel = Panel::default();
        assert_eq!(press_at(&ctx, &mut panel, Pos2::new(RAIL / 2.0, SIZE.y - 30.0)), vec![ToApp::Stop]);
    }

    #[test]
    fn a_pinned_toggle_sends_on_press_and_a_locked_one_doesnt() {
        let ctx = egui::Context::default();
        let tile = |id: &str, locked| Tile { group_id: "g".into(), control_id: id.into(), name: id.into(), kind: TileKind::Toggle { on: false }, locked };
        let mut panel = Panel { page: Page::Pinned, ..Panel::default() };
        panel.state.pinned = vec![tile("ears", false), tile("tail", true)];
        assert_eq!(press_at(&ctx, &mut panel, Pos2::new(RAIL + 40.0, 70.0)), vec![ToApp::Control { group_id: "g".into(), control_id: "ears".into(), level: None }]);
        assert!(press_at(&ctx, &mut panel, Pos2::new(SIZE.x - 40.0, 70.0)).is_empty());
    }

    #[test]
    fn the_rail_switches_pages() {
        let ctx = egui::Context::default();
        let mut panel = Panel::default();
        press_at(&ctx, &mut panel, Pos2::new(RAIL / 2.0, 8.0 + 2.0 * 46.0 + 21.0));
        assert!(panel.page == Page::Pinned);
    }
}

#[cfg(test)]
mod glyphs {
    /// A missing glyph shows as an empty box in the headset.
    #[test]
    fn the_icons_are_in_the_fonts() {
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |_| {});
        for c in ['🏠', '👥', '📌', '🔌', '🔒', '…', '·'] {
            assert!(ctx.fonts(|f| f.has_glyph(&egui::FontId::proportional(20.0), c)), "{c}");
        }
    }
}
