//! The panel's look, in one place: the app's dark Aurora colors with your primary and secondary,
//! and the sizes everything is laid out with. Everything is big: it's poked with a controller tip
//! or hit with a laser.
use egui::Color32;

use crate::wrist::protocol;

pub struct Theme {
    pub bg: Color32,
    pub well: Color32,
    pub line: Color32,
    pub fg: Color32,
    pub muted: Color32,
    /// On states, the current page.
    pub primary: Color32,
    /// Levels.
    pub secondary: Color32,
    /// Touch is driving a toy.
    pub hot: Color32,
    pub ok: Color32,
    pub stop: Color32
}

impl Theme {
    pub fn new(colors: &protocol::Theme) -> Self {
        let rgb = |[r, g, b]: [u8; 3]| Color32::from_rgb(r, g, b);
        Self {
            bg: Color32::from_rgba_premultiplied(17, 20, 31, 235),
            well: Color32::from_rgb(30, 34, 46),
            line: Color32::from_rgb(40, 44, 56),
            fg: Color32::from_rgb(0xee, 0xf0, 0xf7),
            muted: Color32::from_rgb(0x9a, 0xa0, 0xb4),
            primary: rgb(colors.primary),
            secondary: rgb(colors.secondary),
            hot: Color32::from_rgb(0xfb, 0xbf, 0x24),
            ok: Color32::from_rgb(0x4a, 0xde, 0x80),
            stop: Color32::from_rgb(0xf4, 0x3f, 0x5e)
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(&protocol::Theme::default())
    }
}

/// The page's card, at the top.
pub const CARD: egui::Vec2 = egui::Vec2::new(380.0, 270.0);
/// Between the page's card and the bar's.
pub const BAR_GAP: f32 = 8.0;
/// The bar's card under it: the page buttons and Stop.
pub const BAR_HEIGHT: f32 = 64.0;
/// The panel's layout size in points, both cards; it's drawn at a multiple of it.
pub const SIZE: egui::Vec2 = egui::Vec2::new(CARD.x, CARD.y + BAR_GAP + BAR_HEIGHT);
/// Around the page, inside its card.
pub const MARGIN: f32 = 12.0;
/// Between the blocks of a page.
pub const GAP: f32 = 8.0;
/// Between the rows of a list.
pub const LIST_GAP: f32 = 4.0;

pub const HEADER_HEIGHT: f32 = 26.0;
/// A row with a title and a line under it.
pub const ROW_HEIGHT: f32 = 44.0;
/// A row with one line.
pub const LINE_HEIGHT: f32 = 34.0;
pub const STAT_HEIGHT: f32 = 62.0;

pub const ROUND: f32 = 12.0;
pub const ROUND_BIG: f32 = 14.0;
pub const PILL: f32 = 255.0;

pub const TITLE: f32 = 15.0;
pub const TEXT: f32 = 13.0;
pub const SMALL: f32 = 11.0;
pub const BIG: f32 = 22.0;
