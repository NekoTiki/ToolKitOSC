//! What the panel shows, in egui, at 380 × 270 points: the rail on the left with Stop at its
//! bottom, the page on the right. Drawn with the app's dark Aurora colors. Buttons are big: they're
//! poked with a fingertip or hit with a laser.
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use egui::{Align2, Color32, FontId, Pos2, Rect, Rounding, Stroke, Vec2};

use super::protocol::{State, ToApp};

pub const SIZE: Vec2 = Vec2::new(380.0, 270.0);
const RAIL: f32 = 58.0;
const RESUME_HOLD: Duration = Duration::from_millis(1500);

const BG: Color32 = Color32::from_rgba_premultiplied(17, 20, 31, 235);
const WELL: Color32 = Color32::from_rgba_premultiplied(30, 34, 46, 255);
const LINE: Color32 = Color32::from_rgba_premultiplied(40, 44, 56, 255);
const FG: Color32 = Color32::from_rgb(0xee, 0xf0, 0xf7);
const MUTED: Color32 = Color32::from_rgb(0x9a, 0xa0, 0xb4);
const PRIMARY: Color32 = Color32::from_rgb(0xa7, 0x8b, 0xfa);
const OK: Color32 = Color32::from_rgb(0x4a, 0xde, 0x80);
const STOP: Color32 = Color32::from_rgb(0xf4, 0x3f, 0x5e);

#[derive(Default)]
pub struct Panel {
    pub state: State,
    picking_profile: bool,
    /// When the hold on Resume started, and whether it already fired.
    hold: Option<(Instant, bool)>
}

/// Pressed this frame on `rect`: a poke going in or a trigger pull. Fires on the way in, not on
/// release, so Stop is one poke.
fn pressed(ui: &egui::Ui, rect: Rect) -> bool {
    ui.input(|i| i.pointer.primary_pressed() && i.pointer.interact_pos().is_some_and(|p| rect.contains(p)))
}

fn held(ui: &egui::Ui, rect: Rect) -> bool {
    ui.input(|i| i.pointer.primary_down() && i.pointer.interact_pos().is_some_and(|p| rect.contains(p)))
}

fn hovered(ui: &egui::Ui, rect: Rect) -> bool {
    ui.input(|i| i.pointer.hover_pos().is_some_and(|p| rect.contains(p)))
}

fn text(ui: &egui::Ui, pos: Pos2, anchor: Align2, text: &str, size: f32, color: Color32) -> Rect {
    ui.painter().text(pos, anchor, text, FontId::proportional(size), color)
}

/// A row with a title, a muted line under it, and something on its right.
fn row(ui: &egui::Ui, rect: Rect, title: &str, sub: &str) {
    ui.painter().rect_filled(rect, Rounding::same(12.0), WELL);
    text(ui, rect.left_top() + Vec2::new(10.0, 8.0), Align2::LEFT_TOP, title, 13.0, FG);
    text(ui, rect.left_top() + Vec2::new(10.0, 25.0), Align2::LEFT_TOP, sub, 11.0, MUTED);
}

fn button(ui: &egui::Ui, rect: Rect, label: &str, fill: Color32, color: Color32) -> bool {
    let fill = if hovered(ui, rect) { fill.gamma_multiply(1.25) } else { fill };
    ui.painter().rect_filled(rect, Rounding::same(255.0), fill);
    text(ui, rect.center(), Align2::CENTER_CENTER, label, 12.5, color);
    pressed(ui, rect)
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
            again = if self.picking_profile { self.profiles(ui, main, &mut actions) } else { self.home(ui, main, &mut actions) };
        });
        (actions, again)
    }

    fn rail(&mut self, ui: &egui::Ui, actions: &mut Vec<ToApp>) {
        let painter = ui.painter();
        painter.line_segment([Pos2::new(RAIL, 0.0), Pos2::new(RAIL, SIZE.y)], Stroke::new(1.0_f32, LINE));
        let home = Rect::from_min_size(Pos2::new(6.0, 8.0), Vec2::new(RAIL - 12.0, 46.0));
        painter.rect_filled(home, Rounding::same(14.0), PRIMARY.gamma_multiply(0.28));
        painter.rect_stroke(home, Rounding::same(14.0), Stroke::new(1.5_f32, PRIMARY));
        text(ui, home.center(), Align2::CENTER_CENTER, "⌂", 22.0, FG);
        if pressed(ui, home) {
            self.picking_profile = false;
        }

        let stop = Rect::from_min_size(Pos2::new(6.0, SIZE.y - 8.0 - 58.0), Vec2::new(RAIL - 12.0, 58.0));
        let fill = if hovered(ui, stop) { STOP.gamma_multiply(1.2) } else { STOP };
        ui.painter().rect_filled(stop, Rounding::same(16.0), fill);
        text(ui, stop.center(), Align2::CENTER_CENTER, "STOP", 11.0, Color32::WHITE);
        if pressed(ui, stop) {
            actions.push(ToApp::Stop);
        }
    }

    fn home(&mut self, ui: &egui::Ui, main: Rect, actions: &mut Vec<ToApp>) -> bool {
        let state = &self.state;
        text(ui, main.left_top() + Vec2::new(0.0, 4.0), Align2::LEFT_TOP, "ToolKitOSC", 15.0, FG);
        let (chip, color) = if state.paused { ("Paused", STOP) } else { ("Sharing", OK) };
        let galley = ui.painter().layout_no_wrap(chip.into(), FontId::proportional(11.5), color);
        let chip_rect = Rect::from_min_size(Pos2::new(main.right() - galley.size().x - 18.0, main.top() + 2.0), galley.size() + Vec2::new(18.0, 6.0));
        ui.painter().rect_filled(chip_rect, Rounding::same(255.0), color.gamma_multiply(0.18));
        ui.painter().galley(chip_rect.min + Vec2::new(9.0, 3.0), galley, color);

        let mut y = main.top() + 30.0;
        let mut again = false;
        if state.paused {
            // Resume needs a hold, so a stray poke can't undo a Stop.
            let rect = Rect::from_min_size(Pos2::new(main.left(), y), Vec2::new(main.width(), 62.0));
            let down = held(ui, rect);
            let progress = match (&mut self.hold, down) {
                (Some((start, fired)), true) => {
                    let progress = start.elapsed().as_secs_f32() / RESUME_HOLD.as_secs_f32();
                    if progress >= 1.0 && !*fired {
                        *fired = true;
                        actions.push(ToApp::Resume);
                    }
                    progress.min(1.0)
                }
                (hold, true) => {
                    *hold = Some((Instant::now(), false));
                    0.0
                }
                (hold, false) => {
                    *hold = None;
                    0.0
                }
            };
            again = down && progress < 1.0;
            ui.painter().rect_filled(rect, Rounding::same(14.0), WELL);
            let mut fill = rect;
            fill.set_width(rect.width() * progress);
            ui.painter().rect_filled(fill, Rounding::same(14.0), OK.gamma_multiply(0.45));
            text(ui, rect.center() - Vec2::new(0.0, 8.0), Align2::CENTER_CENTER, "Hold to resume", 15.0, FG);
            text(ui, rect.center() + Vec2::new(0.0, 12.0), Align2::CENTER_CENTER, "Viewers can't use your controls", 11.0, MUTED);
            y += 70.0;
        } else {
            let half = (main.width() - 8.0) / 2.0;
            for (i, (big, small)) in [
                (state.watching.to_string(), format!("watching · {} signed in", state.signed_in)),
                (state.recent.to_string(), "actions in the last minute".to_string())
            ]
            .iter()
            .enumerate()
            {
                let rect = Rect::from_min_size(Pos2::new(main.left() + i as f32 * (half + 8.0), y), Vec2::new(half, 62.0));
                ui.painter().rect_filled(rect, Rounding::same(14.0), WELL);
                text(ui, rect.left_top() + Vec2::new(12.0, 8.0), Align2::LEFT_TOP, big, 22.0, FG);
                text(ui, rect.left_top() + Vec2::new(12.0, 40.0), Align2::LEFT_TOP, small, 11.0, MUTED);
            }
            y += 70.0;
        }

        let profile = state.profile.as_ref().and_then(|id| state.profiles.iter().find(|p| &p.id == id)).map_or("No profile", |p| p.name.as_str());
        let rect = Rect::from_min_size(Pos2::new(main.left(), y), Vec2::new(main.width(), 46.0));
        row(ui, rect, "Profile", profile);
        let change = Rect::from_min_size(Pos2::new(rect.right() - 86.0, rect.top() + 8.0), Vec2::new(78.0, 30.0));
        if !state.profiles.is_empty() && button(ui, change, "Change", WELL.gamma_multiply(1.6), FG) {
            self.picking_profile = true;
        }
        y += 54.0;

        if let Some(last) = &state.last {
            let rect = Rect::from_min_size(Pos2::new(main.left(), y), Vec2::new(main.width(), 46.0));
            row(ui, rect, "Last", &last.text);
            text(ui, Pos2::new(rect.right() - 12.0, rect.center().y), Align2::RIGHT_CENTER, &ago(last.at), 11.5, MUTED);
        }
        again
    }

    fn profiles(&mut self, ui: &egui::Ui, main: Rect, actions: &mut Vec<ToApp>) -> bool {
        text(ui, main.left_top() + Vec2::new(0.0, 4.0), Align2::LEFT_TOP, "Profile", 15.0, FG);
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
            text(ui, rect.left_center() + Vec2::new(12.0, 0.0), Align2::LEFT_CENTER, name, 13.0, FG);
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
}

/// Lays the panel out once, for the tests and to check it fits.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_fires_on_the_first_poke() {
        let ctx = egui::Context::default();
        let mut panel = Panel::default();
        let input = |events: Vec<egui::Event>| egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)), events, ..Default::default() };
        let stop = Pos2::new(RAIL / 2.0, SIZE.y - 30.0);
        let _ = ctx.run(input(vec![egui::Event::PointerMoved(stop)]), |ctx| {
            panel.show(ctx);
        });
        let mut actions = Vec::new();
        let _ = ctx.run(
            input(vec![egui::Event::PointerButton { pos: stop, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }]),
            |ctx| actions = panel.show(ctx).0
        );
        assert_eq!(actions, vec![ToApp::Stop]);
    }
}
