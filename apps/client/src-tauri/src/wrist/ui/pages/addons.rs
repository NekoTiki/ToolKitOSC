//! Add-ons: what each add-on that's on says, with its card's dot, then the toys of the avatar
//! you're wearing with their level and whether touch drives them, each under its own title.
//! Dragged up and down to scroll. Read only.
use egui::{Rect, Sense, Ui, Vec2};

use super::Page;
use crate::addons::State as Status;
use crate::wrist::protocol::{AddonRow, State, ToApp, ToyRow};
use crate::wrist::ui::theme::ROW_HEIGHT;
use crate::wrist::ui::widgets::{self, Scroll};
use crate::wrist::ui::Cx;

/// A section's title; the room above it sets the sections apart. The first one needs less.
const TITLE_HEIGHT: f32 = 30.0;
const FIRST_TITLE_HEIGHT: f32 = 16.0;

enum Item<'a> {
    /// With its height.
    Title(&'static str, f32),
    Addon(&'a AddonRow),
    Toy(&'a ToyRow)
}

#[derive(Default)]
pub struct Addons {
    scroll: Scroll
}

impl Page for Addons {
    fn icon(&self) -> &'static str {
        "🔌"
    }

    fn name(&self) -> &'static str {
        "Add-ons"
    }

    fn show(&mut self, ui: &mut Ui, state: &State, cx: &mut Cx) {
        widgets::header(ui, cx, "Add-ons", |_, _| {});
        let mut items = Vec::new();
        let title = |items: &mut Vec<Item>, name| items.push(Item::Title(name, if items.is_empty() { FIRST_TITLE_HEIGHT } else { TITLE_HEIGHT }));
        if !state.addons.is_empty() {
            title(&mut items, "Status");
            items.extend(state.addons.iter().map(Item::Addon));
        }
        if !state.toys.is_empty() {
            title(&mut items, "Toys");
            items.extend(state.toys.iter().map(Item::Toy));
        }
        let height = |item: &Item| if let Item::Title(_, height) = item { *height } else { ROW_HEIGHT };
        widgets::scroll_list(ui, cx, &mut self.scroll, &items, height, "No add-ons are on", |ui, cx, item| match item {
            Item::Title(title, height) => widgets::section(ui, cx, title, *height),
            Item::Addon(addon) => addon_row(ui, cx, addon),
            Item::Toy(toy) => toy_row(ui, cx, toy)
        });
    }
}

fn addon_row(ui: &mut Ui, cx: &mut Cx, addon: &AddonRow) {
    // The colors of the dots on the add-ons' cards in the app.
    let dot = match addon.state {
        Status::Ok => cx.theme.ok,
        Status::Warn | Status::Busy => cx.theme.hot,
        Status::Error => cx.theme.stop,
        Status::Off => cx.theme.muted
    };
    widgets::dot_row(ui, cx, Some(dot), &addon.name, &addon.status, |ui, cx| {
        if addon.launch && widgets::button(ui, cx, "Launch", Vec2::new(78.0, 30.0)) {
            cx.actions.push(ToApp::LaunchIntiface);
        }
        // The leash: how hard it pulls.
        if let Some(level) = addon.level {
            let rect = ui.allocate_exact_size(Vec2::new(66.0, 30.0), Sense::hover()).0;
            widgets::bar(ui, cx, Rect::from_min_size(rect.left_center() - Vec2::new(0.0, 3.0), Vec2::new(rect.width(), 6.0)), level, cx.theme.secondary);
        }
    });
}

fn toy_row(ui: &mut Ui, cx: &mut Cx, toy: &ToyRow) {
    let sub = match (toy.touched, toy.level > 0.0) {
        (true, _) => "Touch is driving it",
        (false, true) => "Running",
        (false, false) => "Idle"
    };
    widgets::row(ui, cx, &toy.name, sub, |ui, cx| {
        let color = if toy.touched { cx.theme.hot } else { cx.theme.secondary };
        widgets::ring(ui, cx, toy.level, color);
    });
}
