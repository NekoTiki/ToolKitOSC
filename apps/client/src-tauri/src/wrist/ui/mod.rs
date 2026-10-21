//! What the panel shows, in egui: two cards, the current page on top and the bar under it, with a
//! button per page and Stop. Once everything is stopped, the whole bar is the button that resumes,
//! after a hold. Pages (see `pages`) stack the shared widgets (see `widgets`) with egui's layout;
//! colors and sizes come from `theme`.
mod pages;
pub mod theme;
mod widgets;

use std::time::{Duration, Instant};

use egui::{Align, Align2, Layout, Pos2, Rect, Rounding, Stroke, UiBuilder, Vec2};

use super::protocol::{State, ToApp};
pub use theme::SIZE;
use theme::{Theme, BAR_GAP, BAR_HEIGHT, CARD, GAP, MARGIN};

/// Resume takes a hold, so a stray poke can't undo a Stop.
const RESUME_HOLD: Duration = Duration::from_millis(1500);
/// The cards' corners.
const CARD_ROUND: f32 = 22.0;

/// What a page gets besides the state: the theme, and where it puts what was pressed.
pub struct Cx<'a> {
    pub theme: &'a Theme,
    pub actions: Vec<ToApp>,
    /// Draw again soon: a hold is filling.
    pub again: bool,
    touch: &'a mut Touch
}

/// What's held or dragged, across frames.
#[derive(Default)]
struct Touch {
    /// What's being held, since when, and whether it already fired.
    hold: Option<(String, Instant, bool)>,
    /// The level last sent for what's being dragged.
    dragging: Option<(String, f64)>,
    /// Where the press now held began.
    origin: Option<Pos2>,
    /// It scrolled something, so letting go isn't a click.
    scrolled: bool,
    /// A press that ended this frame: where it began and where it ended.
    click: Option<(Pos2, Pos2)>,
    /// Where each labelled widget was drawn last frame, for tests to press it by name.
    #[cfg(test)]
    rects: Vec<(String, Rect)>
}

impl Cx<'_> {
    /// Fills a hold on `key` while `down`. Returns how far it is, 0–1, and true the moment it
    /// completes.
    pub fn hold(&mut self, key: &str, down: bool, length: Duration) -> (f32, bool) {
        let hold = &mut self.touch.hold;
        if !down {
            if hold.as_ref().is_some_and(|(held, _, _)| held == key) {
                *hold = None;
            }
            return (0.0, false);
        }
        let (_, start, fired) = match hold {
            Some(hold) if hold.0 == key => hold,
            hold => hold.insert((key.to_owned(), Instant::now(), false))
        };
        let progress = (start.elapsed().as_secs_f32() / length.as_secs_f32()).min(1.0);
        let now = progress >= 1.0 && !*fired;
        *fired |= now;
        self.again |= progress < 1.0;
        (progress, now)
    }

    /// `key` dragged to `level`, 0–1: whether it goes out now. Once it moved `step` since the last
    /// one sent, and always at the ends, so a drag can reach 0 and 100 %.
    pub fn drag(&mut self, key: &str, level: f64, step: f64) -> bool {
        let sent = self.touch.dragging.as_ref().filter(|(dragged, _)| dragged == key).map(|(_, sent)| *sent);
        let moved = sent.map_or(true, |sent| (sent - level).abs() >= step || ((level == 0.0 || level == 1.0) && sent != level));
        if moved {
            self.touch.dragging = Some((key.to_owned(), level));
        }
        moved
    }

    /// Clicked this frame on `rect`: pressed and let go on it, with no scroll in between.
    pub fn clicked(&self, rect: Rect) -> bool {
        self.touch.click.is_some_and(|(from, to)| rect.contains(from) && rect.contains(to))
    }

    /// The press held now scrolled something, so letting go isn't a click.
    pub fn cancel_click(&mut self) {
        self.touch.scrolled = true;
    }

    /// Where the pointer holds down a press that began on `rect`, so a hold or a drag never picks
    /// up a press from somewhere else. With `inside`, only while it's still on `rect`: sliding off
    /// a hold lets go of it, while a drag goes on wherever the pointer goes.
    pub fn held(&self, ui: &egui::Ui, rect: Rect, inside: bool) -> Option<Pos2> {
        if !self.touch.origin.is_some_and(|origin| rect.contains(origin)) {
            return None;
        }
        ui.input(|i| i.pointer.interact_pos().filter(|pos| i.pointer.primary_down() && (!inside || rect.contains(*pos))))
    }

    /// Forgets the press held now, so nothing that appears under it picks it up.
    pub fn forget_press(&mut self) {
        self.touch.origin = None;
    }

    /// Notes where `label` was drawn, for the tests.
    pub fn mark(&mut self, _label: &str, _rect: Rect) {
        #[cfg(test)]
        self.touch.rects.push((_label.to_owned(), _rect));
    }
}

pub struct Panel {
    state: State,
    theme: Theme,
    pages: Vec<Box<dyn pages::Page>>,
    page: usize,
    touch: Touch
}

impl Default for Panel {
    fn default() -> Self {
        Self { state: State::default(), theme: Theme::default(), pages: pages::all(), page: 0, touch: Touch::default() }
    }
}

impl Panel {
    pub fn set_state(&mut self, state: State) {
        if state.theme != self.state.theme {
            self.theme = Theme::new(&state.theme);
        }
        self.state = state;
    }

    /// Draws the panel. Returns what was pressed, and whether it needs to draw again soon (a hold
    /// is filling).
    pub fn show(&mut self, ctx: &egui::Context) -> (Vec<ToApp>, bool) {
        #[cfg(test)]
        self.touch.rects.clear();
        let Self { state, theme, pages, page, touch } = self;
        // Things act when you let go of them, so a poke that slides off cancels; Stop and holds
        // read the pointer themselves.
        touch.click = None;
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, .. } = event {
                    if *pressed {
                        touch.origin = Some(*pos);
                        touch.scrolled = false;
                    } else {
                        touch.click = touch.origin.take().filter(|_| !touch.scrolled).map(|from| (from, *pos));
                    }
                }
            }
        });
        let mut cx = Cx { theme, actions: Vec::new(), again: false, touch };
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            let card = Rect::from_min_size(Pos2::ZERO, CARD);
            let bar_card = Rect::from_min_size(Pos2::new(0.0, CARD.y + BAR_GAP), Vec2::new(CARD.x, BAR_HEIGHT));
            for rect in [card, bar_card] {
                ui.painter().rect_filled(rect, Rounding::same(CARD_ROUND), theme.bg);
                ui.painter().rect_stroke(rect.shrink(0.5), Rounding::same(CARD_ROUND), Stroke::new(1.0_f32, theme.line));
            }
            if state.paused {
                resume(ui, &mut cx, bar_card);
            } else if let Some(picked) = bar(ui, &mut cx, bar_card, pages, *page) {
                pages[*page].leave();
                *page = picked;
            }
            let main = card.shrink2(Vec2::new(14.0, MARGIN));
            let mut ui = ui.new_child(UiBuilder::new().max_rect(main).layout(Layout::top_down(Align::Min)));
            // A little past the page, so the outlines on its edges aren't cut in half.
            ui.set_clip_rect(main.expand(2.0));
            ui.spacing_mut().item_spacing = Vec2::splat(GAP);
            pages[*page].show(&mut ui, state, &mut cx);
            // Nothing held any more: a hold or a drag ends.
            if !ui.input(|i| i.pointer.primary_down()) {
                cx.touch.hold = None;
                cx.touch.dragging = None;
            }
        });
        (cx.actions, cx.again)
    }
}

/// The bar while viewers can use your controls: a button per page, then Stop on the right, with a
/// clear gap so a poke can't land on both. Stop acts the moment it's poked, not when you let go.
/// Returns the page clicked.
fn bar(ui: &egui::Ui, cx: &mut Cx, card: Rect, pages: &[Box<dyn pages::Page>], current: usize) -> Option<usize> {
    let theme = cx.theme;
    let inner = card.shrink(8.0);
    let stop = Rect::from_min_max(Pos2::new(inner.right() - 110.0, inner.top()), inner.max);
    let width = (stop.left() - 16.0 - inner.left() - 6.0 * (pages.len() as f32 - 1.0)) / pages.len() as f32;
    let mut picked = None;
    for (i, page) in pages.iter().enumerate() {
        let rect = Rect::from_min_size(inner.min + Vec2::new(i as f32 * (width + 6.0), 0.0), Vec2::new(width, inner.height()));
        let on = i == current;
        if on {
            ui.painter().rect_filled(rect, Rounding::same(14.0), theme.primary.gamma_multiply(0.28));
            ui.painter().rect_stroke(rect, Rounding::same(14.0), Stroke::new(1.5_f32, theme.primary));
        } else if widgets::hovered(ui, rect) {
            ui.painter().rect_filled(rect, Rounding::same(14.0), theme.well);
        }
        widgets::text(ui, rect.center(), Align2::CENTER_CENTER, page.icon(), 20.0, if on { theme.fg } else { theme.muted }, f32::INFINITY);
        cx.mark(page.name(), rect);
        if cx.clicked(rect) {
            picked = Some(i);
        }
    }

    let fill = if widgets::hovered(ui, stop) { theme.stop.gamma_multiply(1.2) } else { theme.stop };
    ui.painter().rect_filled(stop, Rounding::same(16.0), fill);
    widgets::text(ui, stop.center(), Align2::CENTER_CENTER, "STOP", 15.0, egui::Color32::WHITE, f32::INFINITY);
    cx.mark("STOP", stop);
    if widgets::pressed(ui, stop) {
        cx.actions.push(ToApp::Stop);
        // The poke that stopped isn't a press on what the bar turns into: resuming needs a new one.
        cx.forget_press();
    }
    picked
}

/// The bar once everything is stopped: all of it is the button that resumes, filling up while
/// it's held.
fn resume(ui: &egui::Ui, cx: &mut Cx, card: Rect) {
    let theme = cx.theme;
    let held = cx.held(ui, card, true).is_some();
    let (progress, done) = cx.hold("resume", held, RESUME_HOLD);
    if done {
        cx.actions.push(ToApp::Resume);
    }
    ui.painter().rect_filled(card, Rounding::same(CARD_ROUND), theme.stop.gamma_multiply(0.16));
    let mut fill = card;
    fill.set_width(card.width() * progress);
    ui.painter().with_clip_rect(fill).rect_filled(card, Rounding::same(CARD_ROUND), theme.ok.gamma_multiply(0.45));
    ui.painter().rect_stroke(card.shrink(0.75), Rounding::same(CARD_ROUND), Stroke::new(1.5_f32, if held { theme.ok } else { theme.stop }));
    widgets::text(ui, card.center() - Vec2::new(0.0, 9.0), Align2::CENTER_CENTER, "■  STOPPED · hold to resume", 15.0, theme.fg, card.width() - 24.0);
    widgets::text(ui, card.center() + Vec2::new(0.0, 12.0), Align2::CENTER_CENTER, "Viewers can't use your controls", 11.0, theme.muted, card.width() - 24.0);
    cx.mark("Resume", card);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wrist::protocol::{Tile, TileKind};

    fn input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)), events, ..Default::default() }
    }

    fn button(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() }
    }

    fn frame(ctx: &egui::Context, panel: &mut Panel, events: Vec<egui::Event>) -> Vec<ToApp> {
        let mut actions = Vec::new();
        let _ = ctx.run(input(events), |ctx| actions = panel.show(ctx).0);
        actions
    }

    /// Where `label` was drawn, after a frame to lay the page out.
    fn find(ctx: &egui::Context, panel: &mut Panel, label: &str) -> Rect {
        frame(ctx, panel, Vec::new());
        panel.touch.rects.iter().find(|(name, _)| name == label).unwrap_or_else(|| panic!("no {label}")).1
    }

    /// Moves to `pos` and presses down: what that press sent.
    fn press_down(ctx: &egui::Context, panel: &mut Panel, pos: Pos2) -> Vec<ToApp> {
        frame(ctx, panel, vec![egui::Event::PointerMoved(pos)]);
        frame(ctx, panel, vec![button(pos, true)])
    }

    /// A poke on `label`: what it sent.
    fn press(ctx: &egui::Context, panel: &mut Panel, label: &str) -> Vec<ToApp> {
        let pos = find(ctx, panel, label).center();
        let mut actions = press_down(ctx, panel, pos);
        actions.extend(frame(ctx, panel, vec![button(pos, false)]));
        actions
    }

    fn on_page(name: &str) -> Panel {
        let mut panel = Panel::default();
        panel.page = panel.pages.iter().position(|page| page.name() == name).unwrap();
        panel
    }

    fn tile(id: &str, kind: TileKind, locked: bool) -> Tile {
        Tile { group_id: "g".into(), control_id: id.into(), name: id.into(), kind, locked }
    }

    fn control(id: &str, level: Option<f64>) -> ToApp {
        ToApp::Control { group_id: "g".into(), control_id: id.into(), level, pick: None }
    }

    #[test]
    fn stop_fires_on_the_first_poke() {
        let ctx = egui::Context::default();
        let mut panel = Panel::default();
        assert_eq!(press(&ctx, &mut panel, "STOP"), vec![ToApp::Stop]);
    }

    #[test]
    fn a_pinned_toggle_sends_on_press_and_a_locked_one_doesnt() {
        let ctx = egui::Context::default();
        let mut panel = on_page("Pinned");
        panel.state.pinned.tiles = vec![tile("ears", TileKind::Toggle { on: false }, false), tile("tail", TileKind::Toggle { on: false }, true)];
        assert_eq!(press(&ctx, &mut panel, "ears"), vec![control("ears", None)]);
        assert!(press(&ctx, &mut panel, "tail").is_empty());
    }

    #[test]
    fn a_stop_poke_held_on_doesnt_resume_but_a_new_hold_does() {
        let ctx = egui::Context::default();
        let mut panel = Panel::default();
        let stop = find(&ctx, &mut panel, "STOP").center();
        assert_eq!(press_down(&ctx, &mut panel, stop), vec![ToApp::Stop]);
        // The app says it stopped while the finger is still in: the bar is Resume now.
        panel.state.paused = true;
        std::thread::sleep(RESUME_HOLD + Duration::from_millis(50));
        assert!(frame(&ctx, &mut panel, Vec::new()).is_empty());
        frame(&ctx, &mut panel, vec![button(stop, false)]);
        let resume = find(&ctx, &mut panel, "Resume").center();
        assert!(press_down(&ctx, &mut panel, resume).is_empty());
        std::thread::sleep(RESUME_HOLD + Duration::from_millis(50));
        assert_eq!(frame(&ctx, &mut panel, Vec::new()), vec![ToApp::Resume]);
    }

    #[test]
    fn the_bar_works_again_after_resuming() {
        let ctx = egui::Context::default();
        let mut panel = Panel::default();
        panel.state.paused = true;
        find(&ctx, &mut panel, "Resume");
        panel.state.paused = false;
        press(&ctx, &mut panel, "Pinned");
        assert_eq!(panel.pages[panel.page].name(), "Pinned");
    }

    #[test]
    fn pinned_controls_go_six_a_page() {
        let ctx = egui::Context::default();
        let mut panel = on_page("Pinned");
        panel.state.pinned.tiles = (1..=8).map(|i| tile(&format!("c{i}"), TileKind::Press, false)).collect();
        let shown = |panel: &mut Panel| {
            frame(&ctx, panel, Vec::new());
            panel.touch.rects.iter().filter(|(name, _)| name.starts_with('c')).map(|(name, _)| name.clone()).collect::<Vec<_>>()
        };
        assert_eq!(shown(&mut panel), ["c1", "c2", "c3", "c4", "c5", "c6"]);
        press(&ctx, &mut panel, "›");
        assert_eq!(shown(&mut panel), ["c7", "c8"]);
        assert_eq!(press(&ctx, &mut panel, "c8"), vec![control("c8", None)]);
        press(&ctx, &mut panel, "›");
        assert_eq!(shown(&mut panel)[0], "c1", "wraps around");
    }

    #[test]
    fn a_tile_acts_when_let_go_and_not_if_the_poke_slides_off() {
        let ctx = egui::Context::default();
        let mut panel = on_page("Pinned");
        panel.state.pinned.tiles = vec![tile("ears", TileKind::Toggle { on: false }, false)];
        let pos = find(&ctx, &mut panel, "ears").center();
        assert!(press_down(&ctx, &mut panel, pos).is_empty(), "nothing on the way in");
        assert_eq!(frame(&ctx, &mut panel, vec![button(pos, false)]), vec![control("ears", None)]);
        // Slides off the panel: tracking lets go at 0, 0.
        press_down(&ctx, &mut panel, pos);
        assert!(frame(&ctx, &mut panel, vec![button(Pos2::ZERO, false), egui::Event::PointerGone]).is_empty());
    }

    #[test]
    fn dragging_a_long_list_scrolls_it() {
        let ctx = egui::Context::default();
        let mut panel = on_page("Add-ons");
        let row = |i: usize| crate::wrist::protocol::AddonRow { name: format!("a{i}"), state: crate::addons::State::Ok, status: String::new(), level: None };
        panel.state.addons = (0..10).map(row).collect();
        let last = find(&ctx, &mut panel, "a9");
        assert!(last.top() > CARD.y, "the last row starts out of sight");
        // Two drags up, each staying on the list.
        let start = Pos2::new(CARD.x / 2.0, CARD.y - 30.0);
        for _ in 0..2 {
            press_down(&ctx, &mut panel, start);
            for step in 1..=10 {
                frame(&ctx, &mut panel, vec![egui::Event::PointerMoved(start - Vec2::new(0.0, step as f32 * 16.0))]);
            }
            frame(&ctx, &mut panel, vec![button(start - Vec2::new(0.0, 160.0), false)]);
        }
        let last = find(&ctx, &mut panel, "a9");
        assert!(last.bottom() <= CARD.y, "scrolled to the end: {last:?}");
    }

    #[test]
    fn a_press_from_elsewhere_doesnt_move_a_level_or_fire_a_hold() {
        let ctx = egui::Context::default();
        let mut panel = on_page("Pinned");
        panel.state.pinned.tiles = vec![tile("lush", TileKind::Level { value: 0.2 }, false), tile("shocker", TileKind::Hold, false)];
        let lush = find(&ctx, &mut panel, "lush");
        let shocker = find(&ctx, &mut panel, "shocker");
        // Pressed in the gap under the tiles, then slid onto each.
        let outside = Pos2::new(lush.center().x, CARD.y - 4.0);
        press_down(&ctx, &mut panel, outside);
        assert!(frame(&ctx, &mut panel, vec![egui::Event::PointerMoved(lush.center())]).is_empty());
        frame(&ctx, &mut panel, vec![egui::Event::PointerMoved(shocker.center())]);
        std::thread::sleep(pages::FIRE_HOLD + Duration::from_millis(50));
        assert!(frame(&ctx, &mut panel, Vec::new()).is_empty());
    }

    #[test]
    fn a_level_follows_a_drag_that_leaves_its_tile() {
        let ctx = egui::Context::default();
        let mut panel = on_page("Pinned");
        panel.state.pinned.tiles = vec![tile("lush", TileKind::Level { value: 0.2 }, false)];
        let lush = find(&ctx, &mut panel, "lush");
        press_down(&ctx, &mut panel, lush.center());
        let below = Pos2::new(lush.left() + 2.0, lush.bottom() + 40.0);
        assert_eq!(frame(&ctx, &mut panel, vec![egui::Event::PointerMoved(below)]), vec![control("lush", Some(0.0))]);
    }

    #[test]
    fn a_menu_tile_opens_its_list_and_sends_the_pick() {
        let ctx = egui::Context::default();
        let mut panel = on_page("Pinned");
        let options = vec!["Off".to_owned(), "Wave".to_owned(), "Pulse".to_owned()];
        panel.state.pinned.tiles = vec![tile("lush pattern", TileKind::Menu { options, current: Some(0) }, false)];
        assert!(press(&ctx, &mut panel, "lush pattern").is_empty(), "opening sends nothing");
        let pick = ToApp::Control { group_id: "g".into(), control_id: "lush pattern".into(), level: None, pick: Some(2) };
        assert_eq!(press(&ctx, &mut panel, "Pulse"), vec![pick]);
        press(&ctx, &mut panel, "Back");
        find(&ctx, &mut panel, "lush pattern");
    }

    #[test]
    fn the_rail_switches_pages() {
        let ctx = egui::Context::default();
        let mut panel = Panel::default();
        press(&ctx, &mut panel, "Pinned");
        assert_eq!(panel.pages[panel.page].name(), "Pinned");
    }

    #[test]
    fn a_shock_fires_only_after_its_hold() {
        let ctx = egui::Context::default();
        let mut panel = on_page("Pinned");
        panel.state.pinned.tiles = vec![tile("shocker", TileKind::Hold, false)];
        let pos = find(&ctx, &mut panel, "shocker").center();
        assert!(press_down(&ctx, &mut panel, pos).is_empty());
        std::thread::sleep(pages::FIRE_HOLD + Duration::from_millis(50));
        assert_eq!(frame(&ctx, &mut panel, Vec::new()), vec![control("shocker", None)]);
        assert!(frame(&ctx, &mut panel, Vec::new()).is_empty(), "fires once per hold");
    }

    #[test]
    fn a_dragged_level_reaches_the_end() {
        let ctx = egui::Context::default();
        let mut panel = on_page("Pinned");
        panel.state.pinned.tiles = vec![tile("lush", TileKind::Level { value: 0.2 }, false)];
        let rect = find(&ctx, &mut panel, "lush");
        assert_eq!(press_down(&ctx, &mut panel, rect.right_center() - Vec2::new(2.0, 0.0)), vec![control("lush", Some(1.0))]);
    }

    /// A missing glyph shows as an empty box in the headset.
    #[test]
    fn the_icons_are_in_the_fonts() {
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |_| {});
        let icons = pages::all().iter().map(|page| page.icon()).chain(["🔒", "…", "·", "■", "‹", "›"]).flat_map(str::chars).collect::<Vec<_>>();
        for c in icons {
            assert!(ctx.fonts(|f| f.has_glyph(&egui::FontId::proportional(20.0), c)), "{c}");
        }
    }
}
