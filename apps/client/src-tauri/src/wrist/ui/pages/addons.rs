//! Add-ons: one row per add-on that's on, as its card in the app says, and the toys with their
//! level and whether touch drives them. Read only.
use egui::{Align2, Rect, Sense, Ui, Vec2};

use super::Page;
use crate::wrist::protocol::State;
use crate::wrist::ui::theme::{ROW_HEIGHT, SMALL};
use crate::wrist::ui::widgets;
use crate::wrist::ui::Cx;

pub struct Addons;

impl Page for Addons {
    fn icon(&self) -> &'static str {
        "🔌"
    }

    fn name(&self) -> &'static str {
        "Add-ons"
    }

    fn show(&mut self, ui: &mut Ui, state: &State, cx: &mut Cx) {
        widgets::header(ui, cx, "Add-ons", |_, _| {});
        widgets::list(ui, cx, &state.addons, ROW_HEIGHT, "No add-ons are on", |ui, cx, addon| {
            widgets::row(ui, cx, &addon.name, &addon.status, |ui, cx| {
                if addon.level.is_none() && !addon.touched {
                    return;
                }
                let rect = ui.allocate_exact_size(Vec2::new(66.0, 30.0), Sense::hover()).0;
                if addon.touched {
                    widgets::text(ui, rect.right_top(), Align2::RIGHT_TOP, "Touched", SMALL, cx.theme.hot, rect.width());
                }
                if let Some(level) = addon.level {
                    let color = if addon.touched { cx.theme.hot } else { cx.theme.secondary };
                    widgets::bar(ui, cx, Rect::from_min_size(rect.left_bottom() - Vec2::new(0.0, 6.0), Vec2::new(rect.width(), 6.0)), level, color);
                }
            });
        });
    }
}
