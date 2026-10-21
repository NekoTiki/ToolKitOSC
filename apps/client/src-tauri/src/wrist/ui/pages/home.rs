//! Home: sharing or paused (with Resume after a hold), the viewer count, the profile and the last
//! action.
use std::time::Duration;

use egui::{Ui, Vec2};

use super::Page;
use crate::wrist::protocol::{State, ToApp};
use crate::wrist::ui::theme::LINE_HEIGHT;
use crate::wrist::ui::widgets::{self, ago, entry_text};
use crate::wrist::ui::Cx;

const RESUME_HOLD: Duration = Duration::from_millis(1500);

#[derive(Default)]
pub struct Home {
    /// Showing the profiles to pick from.
    picking: bool
}

impl Page for Home {
    fn icon(&self) -> &'static str {
        "🏠"
    }

    fn name(&self) -> &'static str {
        "Home"
    }

    fn leave(&mut self) {
        self.picking = false;
    }

    fn show(&mut self, ui: &mut Ui, state: &State, cx: &mut Cx) {
        if self.picking {
            return self.profiles(ui, state, cx);
        }
        let home = &state.home;
        widgets::header(ui, cx, "ToolKitOSC", |ui, cx| {
            let (label, color) = if state.paused { ("Paused", cx.theme.stop) } else { ("Sharing", cx.theme.ok) };
            widgets::chip(ui, label, color, 120.0);
        });
        if state.paused {
            // Resume needs a hold, so a stray poke can't undo a Stop.
            if widgets::hold_button(ui, cx, "Hold to resume", "Viewers can't use your controls", RESUME_HOLD) {
                cx.actions.push(ToApp::Resume);
            }
        } else {
            let stats = [
                (home.watching.to_string(), format!("watching · {} signed in", home.signed_in)),
                (home.recent.to_string(), "actions in the last minute".to_owned())
            ];
            widgets::stats(ui, cx, &stats);
        }

        let profile = home.profile.as_ref().and_then(|id| home.profiles.iter().find(|p| &p.id == id)).map_or("No profile", |p| p.name.as_str());
        let mut change = false;
        widgets::row(ui, cx, "Profile", profile, |ui, cx| {
            if !home.profiles.is_empty() {
                change = widgets::button(ui, cx, "Change", Vec2::new(78.0, 30.0));
            }
        });
        self.picking |= change;

        if let Some(last) = state.activity.first() {
            widgets::row(ui, cx, "Last", &entry_text(last), |ui, cx| widgets::note(ui, cx, &ago(last.at)));
        }
    }
}

impl Home {
    fn profiles(&mut self, ui: &mut Ui, state: &State, cx: &mut Cx) {
        widgets::header(ui, cx, "Profile", |_, _| {});
        let home = &state.home;
        let choices: Vec<(Option<&String>, &str)> = std::iter::once((None, "No profile")).chain(home.profiles.iter().map(|p| (Some(&p.id), p.name.as_str()))).collect();
        let mut picked = None;
        widgets::list(ui, cx, &choices, LINE_HEIGHT, "", |ui, cx, (id, name)| {
            if widgets::line(ui, cx, name, "", *id == home.profile.as_ref()) {
                picked = Some(id.cloned());
            }
        });
        if let Some(id) = picked {
            cx.actions.push(ToApp::Profile { id });
            self.picking = false;
        }
    }
}
