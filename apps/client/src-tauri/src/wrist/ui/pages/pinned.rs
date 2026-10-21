//! Pinned: the controls pinned on the avatar you're wearing, as big tiles in a 2 × 3 grid, a page
//! at a time. Toggles, options and presets act on a press, levels fill the tile and are dragged
//! along it, shocks and the OWO Skin fire after a hold. Tiles the profile locks are greyed out.
use std::time::Duration;

use egui::{Align2, Rect, Rounding, Sense, Stroke, Ui, Vec2};

use super::Page;
use crate::wrist::protocol::{State, Tile, TileKind, ToApp};
use crate::wrist::ui::theme::{GAP, ROUND_BIG, SMALL};
use crate::wrist::ui::widgets;
use crate::wrist::ui::Cx;

/// Shocks and the OWO Skin fire after this long a hold.
pub const FIRE_HOLD: Duration = Duration::from_millis(600);
/// A dragged level goes out once it's moved this much.
const LEVEL_STEP: f64 = 0.02;
const COLUMNS: usize = 2;
const ROWS: usize = 3;
const HINT_HEIGHT: f32 = 14.0;
/// See `TileKind::Level` in `show_tile`.
const END: f32 = 6.0;

#[derive(Default)]
pub struct Pinned {
    /// Which 6 it shows.
    page: usize
}

impl Page for Pinned {
    fn icon(&self) -> &'static str {
        "📌"
    }

    fn name(&self) -> &'static str {
        "Pinned"
    }

    fn show(&mut self, ui: &mut Ui, state: &State, cx: &mut Cx) {
        let pinned = &state.pinned;
        let per_page = COLUMNS * ROWS;
        let pages = pinned.tiles.len().div_ceil(per_page).max(1);
        // Unpinned or another avatar since: back to a page that's there.
        self.page = self.page.min(pages - 1);
        widgets::header(ui, cx, "Pinned", |ui, cx| {
            if pages > 1 {
                if widgets::button(ui, cx, "›", Vec2::new(34.0, 26.0)) {
                    self.page = (self.page + 1) % pages;
                }
                widgets::note(ui, cx, &format!("{} / {pages}", self.page + 1));
                if widgets::button(ui, cx, "‹", Vec2::new(34.0, 26.0)) {
                    self.page = (self.page + pages - 1) % pages;
                }
            }
            let width = if pages > 1 { 80.0 } else { 140.0 };
            widgets::chip(ui, pinned.avatar.as_deref().unwrap_or("No avatar"), cx.theme.muted, width);
        });
        if pinned.tiles.is_empty() {
            return widgets::empty(ui, cx, "This avatar has no controls yet");
        }
        let hint = if pinned.nothing_pinned { HINT_HEIGHT + GAP } else { 0.0 };
        let grid = ui.allocate_exact_size(Vec2::new(ui.available_width(), ui.available_height() - hint), Sense::hover()).0;
        let size = Vec2::new((grid.width() - GAP) / COLUMNS as f32, (grid.height() - GAP * (ROWS - 1) as f32) / ROWS as f32);
        for (i, tile) in pinned.tiles.iter().skip(self.page * per_page).take(per_page).enumerate() {
            let at = grid.min + Vec2::new((i % COLUMNS) as f32, (i / COLUMNS) as f32) * (size + Vec2::splat(GAP));
            show_tile(ui, cx, Rect::from_min_size(at, size), tile);
        }
        if pinned.nothing_pinned {
            let rect = ui.allocate_exact_size(Vec2::new(ui.available_width(), HINT_HEIGHT), Sense::hover()).0;
            widgets::text(ui, rect.left_center(), Align2::LEFT_CENTER, "Pin your own: right-click a control, Pin › To your wrist", 10.5, cx.theme.muted, rect.width());
        }
    }
}

fn show_tile(ui: &Ui, cx: &mut Cx, rect: Rect, tile: &Tile) {
    let theme = cx.theme;
    let key = format!("{}/{}", tile.group_id, tile.control_id);
    let usable = !tile.locked && tile.kind != TileKind::AppOnly;
    let on = matches!(tile.kind, TileKind::Toggle { on: true });
    let (fill, border) = match &tile.kind {
        _ if !usable => (theme.well.gamma_multiply(0.6), theme.line),
        TileKind::Toggle { on: true } => (theme.primary.gamma_multiply(0.2), theme.primary.gamma_multiply(0.7)),
        TileKind::Hold => (theme.well, theme.stop.gamma_multiply(0.6)),
        _ if widgets::hovered(ui, rect) => (theme.well.gamma_multiply(1.3), theme.line),
        _ => (theme.well, theme.line)
    };
    ui.painter().rect_filled(rect, Rounding::same(ROUND_BIG), fill);
    ui.painter().rect_stroke(rect, Rounding::same(ROUND_BIG), Stroke::new(1.0_f32, border));
    cx.mark(&tile.name, rect);
    let detail_at = |above: f32, label: &str| widgets::text(ui, rect.left_bottom() + Vec2::new(10.0, -9.0 - above), Align2::LEFT_BOTTOM, label, SMALL, theme.muted, rect.width() - 20.0);
    let detail = |label: &str| detail_at(0.0, label);
    let bar_at = |bottom: f32, height: f32| Rect::from_min_size(rect.left_bottom() + Vec2::new(10.0, -bottom - height), Vec2::new(rect.width() - 20.0, height));
    // A hold lets go when the pointer slides off; a level keeps following it.
    let down = |inside: bool| if usable { cx.held(ui, rect, inside) } else { None };
    let press = |level: Option<f64>| ToApp::Control { group_id: tile.group_id.clone(), control_id: tile.control_id.clone(), level };
    match &tile.kind {
        _ if tile.locked => {
            detail("🔒 Locked");
        }
        TileKind::AppOnly => {
            detail("In the app");
        }
        TileKind::Toggle { .. } | TileKind::Choice { .. } | TileKind::Press => {
            detail(match &tile.kind {
                TileKind::Toggle { .. } if on => "On",
                TileKind::Toggle { .. } => "Off",
                TileKind::Choice { option } => option.as_deref().unwrap_or("Press for the next"),
                _ => "Press to apply"
            });
            if cx.clicked(rect) {
                cx.actions.push(press(None));
            }
        }
        TileKind::Level { value } => {
            // The whole tile is the slider, dragged along; the level shows right away, before the
            // app echoes it.
            let level = match down(false) {
                Some(pos) => {
                    // The last few points at each end are 0 and 100 %, so they're easy to reach.
                    let level = f64::from(((pos.x - rect.left() - END) / (rect.width() - 2.0 * END)).clamp(0.0, 1.0));
                    if cx.drag(&key, level, LEVEL_STEP) {
                        cx.actions.push(press(Some(level)));
                    }
                    level
                }
                None => *value
            };
            let mut filled = rect;
            filled.set_width(rect.width() * level as f32);
            // The tile's own shape, cut at the level, under its name (drawn last).
            ui.painter().with_clip_rect(filled).rect_filled(rect, Rounding::same(ROUND_BIG), theme.secondary.gamma_multiply(0.45));
            widgets::text(ui, rect.left_bottom() + Vec2::new(10.0, -9.0), Align2::LEFT_BOTTOM, &format!("{} %", (level * 100.0).round()), SMALL, theme.fg, rect.width() - 20.0);
        }
        TileKind::Hold => {
            let held = down(true).is_some();
            let (progress, fire) = cx.hold(&key, held, FIRE_HOLD);
            if fire {
                cx.actions.push(press(None));
            }
            // The bar fills under its label.
            widgets::bar(ui, cx, bar_at(10.0, 6.0), f64::from(progress), theme.stop);
            detail_at(12.0, if progress >= 1.0 { "Sent" } else { "Hold to fire" });
        }
    }
    // Last, over a level's fill.
    let color = if usable { theme.fg } else { theme.muted };
    widgets::text(ui, rect.left_top() + Vec2::new(10.0, 8.0), Align2::LEFT_TOP, &tile.name, 12.5, color, rect.width() - 56.0);
}
