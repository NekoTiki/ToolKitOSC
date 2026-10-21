//! Viewers: who's on your share page with their groups and last action, or Activity, what viewers
//! did, newest first.
use egui::{Align2, Pos2, Sense, Ui, Vec2};

use super::Page;
use crate::wrist::protocol::State;
use crate::wrist::ui::theme::{LINE_HEIGHT, ROW_HEIGHT};
use crate::wrist::ui::widgets::{self, ago, entry_text};
use crate::wrist::ui::Cx;

#[derive(Default)]
pub struct Viewers {
    /// Showing Activity instead of who's online.
    activity: bool
}

impl Page for Viewers {
    fn icon(&self) -> &'static str {
        "👥"
    }

    fn name(&self) -> &'static str {
        "Viewers"
    }

    fn show(&mut self, ui: &mut Ui, state: &State, cx: &mut Cx) {
        let mut picked = None;
        widgets::header(ui, cx, "Viewers", |ui, cx| picked = widgets::segmented(ui, cx, &["Online", "Activity"], usize::from(self.activity), 150.0));
        if let Some(picked) = picked {
            self.activity = picked == 1;
        }
        if self.activity {
            return widgets::list(ui, cx, &state.activity, LINE_HEIGHT, "Nothing yet", |ui, cx, entry| {
                widgets::line(ui, cx, &entry_text(entry), &ago(entry.at), false);
            });
        }
        widgets::list(ui, cx, &state.viewers, ROW_HEIGHT, "Nobody on your share page", |ui, cx, viewer| {
            let groups = match (viewer.groups.is_empty(), viewer.signed_in) {
                (false, _) => viewer.groups.join(", "),
                (true, true) => "Signed in".to_owned(),
                (true, false) => "Guest".to_owned()
            };
            let sub = match &viewer.last {
                Some(last) => format!("{groups} · {}", last.what),
                None => groups
            };
            widgets::row(ui, cx, &viewer.name, &sub, |ui, cx| {
                // Online, and how long since their last action.
                let rect = ui.allocate_exact_size(Vec2::new(34.0, 30.0), Sense::hover()).0;
                ui.painter().circle_filled(Pos2::new(rect.right() - 6.0, rect.top() + 3.0), 4.0, cx.theme.ok);
                if let Some(last) = &viewer.last {
                    widgets::text(ui, rect.right_bottom() + Vec2::new(0.0, 2.0), Align2::RIGHT_BOTTOM, &ago(last.at), 10.5, cx.theme.muted, rect.width());
                }
            });
        });
    }
}
