//! The panel's pages, one per rail button. A new page is a file here and a line in `all`.
mod addons;
mod home;
mod pinned;
mod viewers;

use egui::Ui;

use super::Cx;
use crate::wrist::protocol::State;
#[cfg(test)]
pub use pinned::FIRE_HOLD;

pub trait Page {
    /// Its rail button.
    fn icon(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn show(&mut self, ui: &mut Ui, state: &State, cx: &mut Cx);
    /// The rail was pressed: back to how the page starts.
    fn leave(&mut self) {}
}

/// The pages in the rail, top to bottom. The first one shows at start.
pub fn all() -> Vec<Box<dyn Page>> {
    vec![Box::new(home::Home::default()), Box::new(viewers::Viewers::default()), Box::new(pinned::Pinned::default()), Box::new(addons::Addons)]
}
