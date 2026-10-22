//! The HUD: a few small cards in a corner of your view, one per thing it shows (see
//! `crate::overlay::model::HudSettings`), stacked toward that corner and each as wide as what's on
//! it. Display only: nothing on it is pressed.
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use egui::{Align, Align2, Color32, FontId, Layout, Pos2, Rect, Rounding, Sense, Stroke, Ui, UiBuilder, Vec2};

use super::theme::{Theme, GAP, PILL, ROUND_BIG, SMALL};
use super::{widgets, Cx, Touch};
use crate::addons::State as Status;
use crate::overlay::model::Corner;
use crate::wrist::protocol::{Entry, State, ToApp};

/// The HUD's layout size in points, enough for everything at once. The cards take what they need
/// of it, from its corner; the rest stays clear.
pub const HUD_SIZE: Vec2 = Vec2::new(320.0, 250.0);
/// Between the cards, and around them so their outlines aren't cut.
const SPACE: f32 = 6.0;
const EDGE: f32 = 1.0;
const TEXT: f32 = 14.0;
const ROW: f32 = 34.0;
const TOYS_ROW: f32 = 44.0;
/// Inside a card.
const PAD: Vec2 = Vec2::new(12.0, 6.0);
/// The last action stays this long, then fades out over `FADE`.
const LAST_FOR: Duration = Duration::from_secs(6);
const FADE: Duration = Duration::from_secs(1);
const MAX_TOYS: usize = 4;
const MAX_WARNINGS: usize = 2;

#[derive(Default)]
pub struct Hud {
    state: State,
    theme: Theme
}

impl Hud {
    pub fn set_state(&mut self, state: State) {
        if state.theme != self.state.theme {
            self.theme = Theme::new(&state.theme);
        }
        self.state = state;
    }

    /// Draws the HUD. Returns nothing pressed, and whether to draw again soon (the last action is
    /// fading).
    pub fn show(&mut self, ctx: &egui::Context) -> (Vec<ToApp>, bool) {
        let mut touch = Touch::default();
        let mut cx = Cx { theme: &self.theme, actions: Vec::new(), again: false, touch: &mut touch };
        let (state, hud) = (&self.state, &self.state.hud);
        let right = matches!(hud.corner, Corner::BottomRight | Corner::TopRight);
        let bottom = matches!(hud.corner, Corner::BottomLeft | Corner::BottomRight);

        let mut rows: Vec<(f32, f32, Row)> = Vec::new();
        if hud.status || hud.viewers {
            rows.push((ROW, 1.0, Row::Status));
        }
        if let Some(entry) = state.activity.first().filter(|_| hud.last_action) {
            let age = Duration::from_millis(now_ms().saturating_sub(entry.at));
            if age < LAST_FOR + FADE {
                let fading = age.saturating_sub(LAST_FOR);
                cx.again |= fading > Duration::ZERO;
                rows.push((ROW, 1.0 - fading.as_secs_f32() / FADE.as_secs_f32(), Row::Last(entry)));
            }
        }
        if hud.leash && state.leash.is_some() {
            rows.push((ROW, 1.0, Row::Leash));
        }
        if hud.toys && !state.toys.is_empty() {
            rows.push((TOYS_ROW, 1.0, Row::Toys));
        }
        if hud.warnings {
            let warnings = state.addons.iter().filter(|addon| matches!(addon.state, Status::Warn | Status::Error));
            rows.extend(warnings.take(MAX_WARNINGS).map(|addon| (ROW, 1.0, Row::Warning(&addon.name, &addon.status, addon.state == Status::Error))));
        }

        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            let total = rows.iter().map(|(height, _, _)| height + SPACE).sum::<f32>() - SPACE;
            let mut top = if bottom { HUD_SIZE.y - EDGE - total } else { EDGE };
            for (height, alpha, row) in &rows {
                card(ui, &mut cx, top, *height, right, *alpha, |ui, cx| row.show(ui, cx, state));
                top += height + SPACE;
            }
        });
        (cx.actions, cx.again)
    }
}

enum Row<'a> {
    /// Sharing or Stopped, and the viewers.
    Status,
    Last(&'a Entry),
    Leash,
    Toys,
    /// An add-on's name and status, and whether it's an error.
    Warning(&'a str, &'a str, bool)
}

impl Row<'_> {
    fn show(&self, ui: &mut Ui, cx: &mut Cx, state: &State) {
        let theme = cx.theme;
        match self {
            Row::Status => {
                if state.hud.status {
                    let (text, color) = if state.paused { ("Stopped", theme.stop) } else { ("Sharing", theme.ok) };
                    pill(ui, text, color);
                }
                if state.hud.viewers {
                    let home = &state.home;
                    label(ui, &format!("{} watching", home.watching), TEXT, theme.fg, 140.0);
                    if home.signed_in > 0 {
                        label(ui, &format!("{} signed in", home.signed_in), SMALL, theme.muted, 90.0);
                    }
                }
            }
            Row::Last(entry) => {
                label(ui, &widgets::entry_text(entry), TEXT, theme.fg, 230.0);
                label(ui, &widgets::ago(entry.at), SMALL, theme.muted, 50.0);
            }
            Row::Leash => {
                let Some(pull) = &state.leash else { return };
                arrow(ui, theme, pull.right as f32, pull.forward as f32);
                label(ui, &format!("{} pulls", pull.who), TEXT, theme.fg, 160.0);
                let rect = ui.allocate_exact_size(Vec2::new(56.0, 6.0), Sense::hover()).0;
                widgets::bar(ui, cx, rect, pull.stretch, theme.secondary);
            }
            Row::Toys => {
                for toy in state.toys.iter().take(MAX_TOYS) {
                    widgets::ring(ui, cx, toy.level, if toy.touched { theme.hot } else { theme.secondary });
                    label(ui, &toy.name, SMALL, theme.muted, 60.0);
                }
            }
            Row::Warning(name, status, error) => {
                let rect = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover()).0;
                ui.painter().circle_filled(rect.center(), 4.0, if *error { theme.stop } else { theme.hot });
                label(ui, name, TEXT, theme.fg, 110.0);
                label(ui, status, SMALL, theme.muted, 150.0);
            }
        }
    }
}

/// A card `height` tall at `top`, as wide as what `add` puts in it, against the HUD's left or
/// right edge, and faded to `alpha`.
fn card(ui: &mut Ui, cx: &mut Cx, top: f32, height: f32, right: bool, alpha: f32, add: impl Fn(&mut Ui, &mut Cx)) {
    let room = Rect::from_min_size(Pos2::new(EDGE, top), Vec2::new(HUD_SIZE.x - 2.0 * EDGE, height)).shrink2(PAD);
    let line = |ui: &mut Ui, rect: Rect, builder: UiBuilder| {
        let mut child = ui.new_child(builder.max_rect(rect).layout(Layout::left_to_right(Align::Center)));
        child.spacing_mut().item_spacing = Vec2::splat(GAP);
        child.multiply_opacity(alpha);
        child
    };
    // Laid out once unseen to know its width, then for real where that puts it.
    let mut sizing = line(ui, room, UiBuilder::new().invisible());
    add(&mut sizing, cx);
    let width = sizing.min_rect().width() + 2.0 * PAD.x;
    let left = if right { HUD_SIZE.x - EDGE - width } else { EDGE };
    let rect = Rect::from_min_size(Pos2::new(left, top), Vec2::new(width, height));
    ui.painter().rect_filled(rect, Rounding::same(ROUND_BIG), cx.theme.bg.gamma_multiply(alpha));
    ui.painter().rect_stroke(rect.shrink(0.5), Rounding::same(ROUND_BIG), Stroke::new(1.0_f32, cx.theme.line.gamma_multiply(alpha)));
    let mut inner = line(ui, rect.shrink2(PAD), UiBuilder::new());
    add(&mut inner, cx);
}

/// One line of text, as wide as it is (at most `max`), in the row's layout.
fn label(ui: &mut Ui, text: &str, size: f32, color: Color32, max: f32) {
    let galley = egui::WidgetText::from(text).into_galley(ui, Some(egui::TextWrapMode::Truncate), max, FontId::proportional(size));
    let rect = ui.allocate_exact_size(galley.size(), Sense::hover()).0;
    ui.painter().galley(rect.min, galley, color);
}

/// "Sharing" or "Stopped": a dot and the word on a tinted pill.
fn pill(ui: &mut Ui, text: &str, color: Color32) {
    let galley = egui::WidgetText::from(text).into_galley(ui, None, f32::INFINITY, FontId::proportional(TEXT));
    let rect = ui.allocate_exact_size(Vec2::new(galley.size().x + 32.0, 24.0), Sense::hover()).0;
    ui.painter().rect_filled(rect, Rounding::same(PILL), color.gamma_multiply(0.2));
    ui.painter().circle_filled(rect.left_center() + Vec2::new(12.0, 0.0), 4.0, color);
    ui.painter().galley(Align2::LEFT_CENTER.anchor_size(rect.left_center() + Vec2::new(22.0, 0.0), galley.size()).min, galley, color);
}

/// An arrow toward where the leash walks you: up is forward. A dot while it only holds you.
fn arrow(ui: &mut Ui, theme: &Theme, right: f32, forward: f32) {
    let rect = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover()).0;
    let center = rect.center();
    let length = (right * right + forward * forward).sqrt();
    if length < 0.05 {
        ui.painter().circle_filled(center, 4.0, theme.secondary);
        return;
    }
    // Screen y points down.
    let dir = Vec2::new(right, -forward) / length;
    let (tip, tail) = (center + dir * 9.0, center - dir * 9.0);
    let stroke = Stroke::new(3.0_f32, theme.secondary);
    ui.painter().line_segment([tail, tip], stroke);
    let side = Vec2::new(-dir.y, dir.x);
    ui.painter().line_segment([tip, tip - dir * 6.0 + side * 5.0], stroke);
    ui.painter().line_segment([tip, tip - dir * 6.0 - side * 5.0], stroke);
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wrist::protocol::Pull;

    /// The shapes a frame draws, outside the HUD's corner cards.
    fn draw(hud: &mut Hud) -> Vec<Rect> {
        let ctx = egui::Context::default();
        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, HUD_SIZE)), ..Default::default() };
        let output = ctx.run(input, |ctx| {
            hud.show(ctx);
        });
        output.shapes.iter().map(|shape| shape.shape.visual_bounding_rect()).filter(|rect| rect.is_positive()).collect()
    }

    #[test]
    fn it_only_draws_what_is_on_and_hugs_its_corner() {
        let mut hud = Hud::default();
        hud.state.hud = crate::overlay::model::HudSettings { status: false, viewers: false, last_action: false, warnings: false, ..Default::default() };
        assert!(draw(&mut hud).is_empty());

        hud.state.hud.status = true;
        let drawn = draw(&mut hud);
        let bounds = drawn.iter().fold(Rect::NOTHING, |all, rect| all.union(*rect));
        // Bottom left by default: a small pill there, the rest clear.
        assert!(bounds.left() < 2.0 && bounds.bottom() > HUD_SIZE.y - 2.0, "{bounds:?}");
        assert!(bounds.width() < HUD_SIZE.x / 2.0 && bounds.height() <= ROW + 1.0, "{bounds:?}");

        hud.state.hud.corner = Corner::TopRight;
        hud.state.hud.leash = true;
        hud.state.leash = Some(Pull { who: "Sam".into(), stretch: 0.5, right: 0.0, forward: 1.0 });
        let bounds = draw(&mut hud).iter().fold(Rect::NOTHING, |all, rect| all.union(*rect));
        assert!(bounds.right() > HUD_SIZE.x - 2.0 && bounds.top() < 2.0, "{bounds:?}");
        assert!(bounds.height() > ROW * 2.0, "{bounds:?}");
    }

    #[test]
    fn the_last_action_goes_away_after_a_while() {
        let mut hud = Hud::default();
        hud.state.hud = crate::overlay::model::HudSettings { status: false, viewers: false, warnings: false, ..Default::default() };
        hud.state.activity = vec![Entry { at: now_ms(), who: "Sam".into(), what: "turned on Ears".into() }];
        assert!(!draw(&mut hud).is_empty());
        hud.state.activity[0].at = now_ms() - (LAST_FOR + FADE).as_millis() as u64 - 10;
        assert!(draw(&mut hud).is_empty());
    }
}
