//! The pieces pages are made of. Each takes its room from the page's layout (top to bottom) and
//! paints itself in the theme's look.
//!
//! Presses don't go through egui's widget interaction: egui only knows where widgets were the frame
//! before, while the panel draws a frame only when something changes, and its clicks give up after
//! a few points of movement, less than a finger shakes. Things act on a click (`Cx::clicked`):
//! pressed and let go on them. Stop acts the moment it's poked (`pressed`).
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use egui::{Align, Align2, Color32, FontId, Layout, Pos2, Rect, Rounding, Sense, Ui, UiBuilder, Vec2};

use super::theme::*;
use super::Cx;
use crate::wrist::protocol::Entry;

/// Pressed this frame on `rect`: a poke going in or a trigger pull. Only for Stop: everything else
/// acts when it's let go (`Cx::clicked`).
pub fn pressed(ui: &Ui, rect: Rect) -> bool {
    ui.input(|i| i.pointer.primary_pressed() && i.pointer.interact_pos().is_some_and(|p| rect.contains(p)))
}

pub fn hovered(ui: &Ui, rect: Rect) -> bool {
    ui.input(|i| i.pointer.hover_pos().is_some_and(|p| rect.contains(p)))
}

/// One line of text at `pos`, cut to `width` with an ellipsis. Returns where it went.
pub fn text(ui: &Ui, pos: Pos2, anchor: Align2, text: &str, size: f32, color: Color32, width: f32) -> Rect {
    let galley = egui::WidgetText::from(text).into_galley(ui, Some(egui::TextWrapMode::Truncate), width.max(0.0), FontId::proportional(size));
    let rect = anchor.anchor_size(pos, galley.size());
    ui.painter().galley(rect.min, galley, color);
    rect
}

/// Takes the page's full width, `height` tall.
fn take(ui: &mut Ui, width: f32, height: f32) -> Rect {
    ui.allocate_exact_size(Vec2::new(width, height), Sense::hover()).0
}

/// Lays out `add` right to left inside `rect`; returns how wide it came out.
fn right_side(ui: &mut Ui, cx: &mut Cx, rect: Rect, add: impl FnOnce(&mut Ui, &mut Cx)) -> f32 {
    let mut right = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::right_to_left(Align::Center)));
    right.spacing_mut().item_spacing = Vec2::splat(GAP);
    add(&mut right, cx);
    let used = right.min_rect().width();
    if used > 0.0 { used + GAP } else { 0.0 }
}

/// The page's title, with room on its right for a chip or a switch.
pub fn header(ui: &mut Ui, cx: &mut Cx, title: &str, right: impl FnOnce(&mut Ui, &mut Cx)) {
    let rect = take(ui, ui.available_width(), HEADER_HEIGHT);
    let used = right_side(ui, cx, rect, right);
    text(ui, rect.left_center(), Align2::LEFT_CENTER, title, TITLE, cx.theme.fg, rect.width() - used);
}

/// A small pill of `color`, at most `width` wide.
pub fn chip(ui: &mut Ui, label: &str, color: Color32, width: f32) {
    let galley = egui::WidgetText::from(label).into_galley(ui, Some(egui::TextWrapMode::Truncate), width - 18.0, FontId::proportional(11.5));
    let rect = take(ui, galley.size().x + 18.0, galley.size().y + 6.0);
    ui.painter().rect_filled(rect, Rounding::same(PILL), color.gamma_multiply(0.18));
    ui.painter().galley(rect.min + Vec2::new(9.0, 3.0), galley, color);
}

/// Small muted text in the layout, as wide as it is, like "8 s".
pub fn note(ui: &mut Ui, cx: &mut Cx, label: &str) {
    let galley = egui::WidgetText::from(label).into_galley(ui, Some(egui::TextWrapMode::Truncate), ui.available_width().min(160.0), FontId::proportional(SMALL));
    let rect = take(ui, galley.size().x, 16.0);
    ui.painter().galley(Align2::LEFT_CENTER.anchor_size(rect.left_center(), galley.size()).min, galley, cx.theme.muted);
}

/// Muted text where a list would be.
pub fn empty(ui: &mut Ui, cx: &mut Cx, label: &str) {
    let rect = take(ui, ui.available_width(), 24.0);
    text(ui, rect.left_center(), Align2::LEFT_CENTER, label, TEXT, cx.theme.muted, rect.width());
}

/// A row with a title, a muted line under it, and `right` on its right.
pub fn row(ui: &mut Ui, cx: &mut Cx, title: &str, sub: &str, right: impl FnOnce(&mut Ui, &mut Cx)) {
    dot_row(ui, cx, None, title, sub, right);
}

/// A `row` with a dot of `dot`'s color on its left, like an add-on's status.
pub fn dot_row(ui: &mut Ui, cx: &mut Cx, dot: Option<Color32>, title: &str, sub: &str, right: impl FnOnce(&mut Ui, &mut Cx)) {
    let rect = take(ui, ui.available_width(), ROW_HEIGHT);
    cx.mark(title, rect);
    ui.painter().rect_filled(rect, Rounding::same(ROUND), cx.theme.well);
    let indent = match dot {
        Some(color) => {
            ui.painter().circle_filled(rect.left_center() + Vec2::new(14.0, 0.0), 4.0, color);
            24.0
        }
        None => 10.0
    };
    let used = right_side(ui, cx, rect.shrink2(Vec2::new(10.0, 6.0)), right);
    let width = rect.width() - indent - 10.0 - used;
    text(ui, rect.left_top() + Vec2::new(indent, 7.0), Align2::LEFT_TOP, title, TEXT, cx.theme.fg, width);
    text(ui, rect.left_top() + Vec2::new(indent, 24.0), Align2::LEFT_TOP, sub, SMALL, cx.theme.muted, width);
}

/// A one-line row, highlighted when `on`, with muted text on its right. Returns whether it was
/// pressed.
pub fn line(ui: &mut Ui, cx: &mut Cx, label: &str, right: &str, on: bool) -> bool {
    let theme = cx.theme;
    let rect = take(ui, ui.available_width(), LINE_HEIGHT);
    let fill = if on { theme.primary.gamma_multiply(0.3) } else { theme.well };
    ui.painter().rect_filled(rect, Rounding::same(ROUND), fill);
    let used = if right.is_empty() { 0.0 } else { text(ui, rect.right_center() - Vec2::new(10.0, 0.0), Align2::RIGHT_CENTER, right, SMALL, theme.muted, 80.0).width() + GAP };
    text(ui, rect.left_center() + Vec2::new(10.0, 0.0), Align2::LEFT_CENTER, label, TEXT, theme.fg, rect.width() - 20.0 - used);
    cx.mark(label, rect);
    cx.clicked(rect)
}

/// Big numbers side by side, each with a muted line under it.
pub fn stats(ui: &mut Ui, cx: &mut Cx, stats: &[(String, String)]) {
    let rect = take(ui, ui.available_width(), STAT_HEIGHT);
    let width = (rect.width() - GAP * (stats.len() as f32 - 1.0)) / stats.len() as f32;
    for (i, (big, small)) in stats.iter().enumerate() {
        let stat = Rect::from_min_size(rect.min + Vec2::new(i as f32 * (width + GAP), 0.0), Vec2::new(width, rect.height()));
        ui.painter().rect_filled(stat, Rounding::same(ROUND_BIG), cx.theme.well);
        text(ui, stat.left_top() + Vec2::new(12.0, 8.0), Align2::LEFT_TOP, big, BIG, cx.theme.fg, width - 24.0);
        text(ui, stat.left_top() + Vec2::new(12.0, 40.0), Align2::LEFT_TOP, small, SMALL, cx.theme.muted, width - 18.0);
    }
}

pub fn button(ui: &mut Ui, cx: &mut Cx, label: &str, size: Vec2) -> bool {
    let rect = take(ui, size.x, size.y);
    let fill = cx.theme.well.gamma_multiply(1.6);
    let fill = if hovered(ui, rect) { fill.gamma_multiply(1.25) } else { fill };
    ui.painter().rect_filled(rect, Rounding::same(PILL), fill);
    text(ui, rect.center(), Align2::CENTER_CENTER, label, 12.5, cx.theme.fg, rect.width());
    cx.mark(label, rect);
    cx.clicked(rect)
}

/// A pill of choices, `width` wide; returns the one pressed.
pub fn segmented(ui: &mut Ui, cx: &mut Cx, labels: &[&str], on: usize, width: f32) -> Option<usize> {
    let theme = cx.theme;
    let rect = take(ui, width, HEADER_HEIGHT);
    ui.painter().rect_filled(rect, Rounding::same(PILL), theme.well);
    let part_width = rect.width() / labels.len() as f32;
    let mut picked = None;
    for (i, label) in labels.iter().enumerate() {
        let part = Rect::from_min_size(rect.min + Vec2::new(i as f32 * part_width, 0.0), Vec2::new(part_width, rect.height())).shrink(3.0);
        if i == on {
            ui.painter().rect_filled(part, Rounding::same(PILL), theme.primary.gamma_multiply(0.35));
        }
        text(ui, part.center(), Align2::CENTER_CENTER, label, 11.5, if i == on { theme.fg } else { theme.muted }, part.width());
        cx.mark(label, part);
        if cx.clicked(part) {
            picked = Some(i);
        }
    }
    picked
}

/// A thin bar in `rect`, filled to `level`, 0–1.
pub fn bar(ui: &Ui, cx: &Cx, rect: Rect, level: f64, color: Color32) {
    ui.painter().rect_filled(rect, Rounding::same(PILL), cx.theme.line);
    let mut fill = rect;
    fill.set_width(rect.width() * level.clamp(0.0, 1.0) as f32);
    ui.painter().rect_filled(fill, Rounding::same(PILL), color);
}

/// A wide button that acts only after it's held for `length`, filling up meanwhile. Returns true
/// the moment it completes.
pub fn hold_button(ui: &mut Ui, cx: &mut Cx, title: &str, sub: &str, length: Duration) -> bool {
    let rect = take(ui, ui.available_width(), STAT_HEIGHT);
    let (progress, done) = cx.hold(title, cx.held(ui, rect, true).is_some(), length);
    ui.painter().rect_filled(rect, Rounding::same(ROUND_BIG), cx.theme.well);
    let mut fill = rect;
    fill.set_width(rect.width() * progress);
    ui.painter().rect_filled(fill, Rounding::same(ROUND_BIG), cx.theme.ok.gamma_multiply(0.45));
    text(ui, rect.center() - Vec2::new(0.0, 8.0), Align2::CENTER_CENTER, title, TITLE, cx.theme.fg, rect.width());
    text(ui, rect.center() + Vec2::new(0.0, 12.0), Align2::CENTER_CENTER, sub, SMALL, cx.theme.muted, rect.width());
    cx.mark(title, rect);
    done
}

/// One row per item, `height` tall, until the page runs out of room; the last one that fits then
/// says how many more there are.
pub fn list<T>(ui: &mut Ui, cx: &mut Cx, items: &[T], height: f32, empty_label: &str, mut each: impl FnMut(&mut Ui, &mut Cx, &T)) {
    if items.is_empty() {
        return empty(ui, cx, empty_label);
    }
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = LIST_GAP;
        for (i, item) in items.iter().enumerate() {
            let left = items.len() - i;
            if left > 1 && ui.available_height() < height + LIST_GAP + 16.0 {
                note(ui, cx, &format!("+{left} more"));
                break;
            }
            each(ui, cx, item);
        }
    });
}

/// Where a `scroll_list` is, kept by its page across frames.
#[derive(Default)]
pub struct Scroll {
    offset: f32,
    /// Where the pointer held the list last frame, while it drags it.
    grab: Option<f32>,
    /// How far this drag moved it.
    moved: f32
}

/// A press that moves this far on a list scrolls it instead of clicking.
const SCROLL_SLOP: f32 = 8.0;

/// One row per item, `height` tall, in the rest of the page, dragged up and down to scroll, with a
/// thin bar on the right while it doesn't all fit.
pub fn scroll_list<T>(ui: &mut Ui, cx: &mut Cx, scroll: &mut Scroll, items: &[T], height: f32, empty_label: &str, mut each: impl FnMut(&mut Ui, &mut Cx, &T)) {
    if items.is_empty() {
        return empty(ui, cx, empty_label);
    }
    let area = take(ui, ui.available_width(), ui.available_height());
    let content = items.len() as f32 * (height + LIST_GAP) - LIST_GAP;
    let max = (content - area.height()).max(0.0);
    match cx.held(ui, area, false) {
        Some(pos) => {
            if let Some(last) = scroll.grab {
                scroll.offset -= pos.y - last;
                scroll.moved += (pos.y - last).abs();
                if scroll.moved > SCROLL_SLOP {
                    cx.cancel_click();
                }
            }
            scroll.grab = Some(pos.y);
        }
        None => {
            scroll.grab = None;
            scroll.moved = 0.0;
        }
    }
    // Clamped every frame: the list may have got shorter.
    scroll.offset = scroll.offset.clamp(0.0, max);

    let bar = if max > 0.0 { 8.0 } else { 0.0 };
    let rows = Rect::from_min_size(area.min - Vec2::new(0.0, scroll.offset), Vec2::new(area.width() - bar, content));
    let mut list = ui.new_child(UiBuilder::new().max_rect(rows).layout(Layout::top_down(Align::Min)));
    list.set_clip_rect(area.expand(2.0).intersect(ui.clip_rect()));
    list.spacing_mut().item_spacing.y = LIST_GAP;
    for item in items {
        each(&mut list, cx, item);
    }
    if max > 0.0 {
        let track = Rect::from_min_max(Pos2::new(area.right() - 3.0, area.top()), area.right_bottom());
        let thumb = (area.height() * area.height() / content).max(16.0);
        let top = track.top() + (track.height() - thumb) * scroll.offset / max;
        ui.painter().rect_filled(track, Rounding::same(PILL), cx.theme.well);
        ui.painter().rect_filled(Rect::from_min_size(Pos2::new(track.left(), top), Vec2::new(track.width(), thumb)), Rounding::same(PILL), cx.theme.muted);
    }
}

/// "8 s", "2 min", "1 h": how long ago `at`, in Unix ms.
pub fn ago(at: u64) -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
    let seconds = now.saturating_sub(at) / 1000;
    match seconds {
        0..=59 => format!("{seconds} s"),
        60..=3599 => format!("{} min", seconds / 60),
        _ => format!("{} h", seconds / 3600)
    }
}

/// "Sam turned on Ears".
pub fn entry_text(entry: &Entry) -> String {
    format!("{} {}", entry.who, entry.what)
}
