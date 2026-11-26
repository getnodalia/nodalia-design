//! The window frame of the enterprise apps when they draw their own (Linux, unless the person
//! picked the desktop's): the empty part of the top bar moves the window, a double click maximizes
//! it, the edges resize it, and the NDL controls sit at the desktop's side. Windows keeps its native
//! frame. macOS keeps its native traffic lights, but inside the window: the content runs under a
//! transparent title bar, the top bar leaves room for the lights on its left, and its empty part
//! moves the window.
//!
//! Ágora and Hito used to carry a copy each; this is the only one.

use egui::{pos2, CursorIcon, PointerButton, Rect, ResizeDirection, Sense, Ui, ViewportCommand};

use crate::titlebar::{Placement, Side};
use crate::{space, window_controls, Mode, Palette};

/// Whether this platform may draw NDL window controls at all.
pub const NDL_CONTROLS_POSSIBLE: bool = cfg!(target_os = "linux");

/// Whether the window's content runs under a transparent title bar (macOS).
pub const INSET_TITLE_BAR: bool = cfg!(target_os = "macos");

/// Room the macOS traffic lights take at the left of the top bar, with the gap to the first
/// control.
pub const TRAFFIC_LIGHTS_WIDTH: f32 = crate::titlebar::MAC_LIGHTS;

/// Width the controls take at the right of the bar, margin included.
pub const CONTROLS_WIDTH: f32 = window_controls::WIDTH + space::SM;

const EDGE: f32 = 5.0;

/// The native frame's options for this platform: on macOS the content fills the window and the
/// traffic lights float over the top bar; elsewhere the window has decorations unless the app draws
/// the NDL ones.
pub fn frame(viewport: egui::ViewportBuilder, ndl_controls: bool) -> egui::ViewportBuilder {
    if INSET_TITLE_BAR {
        viewport
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
            .with_titlebar_buttons_shown(true)
    } else {
        viewport.with_decorations(!ndl_controls)
    }
}

/// Space to leave at the left of the top bar before its first widget.
pub fn leading_inset() -> f32 {
    if INSET_TITLE_BAR {
        TRAFFIC_LIGHTS_WIDTH
    } else {
        0.0
    }
}

/// Whether the app moves the window from the top bar itself (it draws the frame, or the native
/// title bar is hidden behind the content).
pub fn drags_window(ndl_controls: bool) -> bool {
    ndl_controls || INSET_TITLE_BAR
}

/// Make the empty area of the bar drag and maximize the window. Call before drawing the bar's own
/// widgets, so they sit on top of it.
pub fn drag_area(ui: &Ui, rect: Rect) {
    let response = ui.interact(
        rect,
        ui.scope_id().with("nodalia_enterprise_title_drag"),
        Sense::click_and_drag(),
    );
    if response.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if response.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
}

/// The end of the bar the NDL window controls sit at: the side the desktop keeps its own on (the
/// right unless the desktop's button layout puts them at the left).
pub fn side() -> Side {
    crate::titlebar::system_side()
}

/// Draw the NDL window controls at the desktop's side of `row` and act on them.
pub fn controls(
    ui: &Ui,
    row: Rect,
    palette: &Palette,
    mode: Mode,
    labels: window_controls::Labels<'_>,
) {
    let (focused, maximized) = ui.input(|i| (i.focused, i.viewport().maximized.unwrap_or(false)));
    let side = side();
    let rect = Placement::nodalia(side).controls_rect(row);
    if let Some(action) =
        window_controls::show_on(ui, rect, side, palette, mode, focused, maximized, labels)
    {
        ui.ctx().send_viewport_cmd(command(action));
    }
}

/// The viewport command an NDL window control stands for.
pub fn command(action: window_controls::Action) -> ViewportCommand {
    use window_controls::Action;
    match action {
        Action::Minimize => ViewportCommand::Minimized(true),
        Action::Maximize => ViewportCommand::Maximized(true),
        Action::Restore => ViewportCommand::Maximized(false),
        Action::Close => ViewportCommand::Close,
    }
}

/// Resize handles along the window's edges, when it is not maximized.
pub fn resize_edges(ui: &Ui, full: Rect) {
    if ui.input(|i| {
        i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)
    }) {
        return;
    }
    for (index, (direction, cursor, rect)) in bands(full).into_iter().enumerate() {
        let response = ui
            .interact(
                rect,
                ui.scope_id().with(("nodalia_enterprise_resize", index)),
                Sense::drag(),
            )
            .on_hover_cursor(cursor);
        if response.drag_started_by(PointerButton::Primary) {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::BeginResize(direction));
        }
    }
}

/// The eight resize bands around `full`: where each one is and what it resizes.
pub fn bands(full: Rect) -> [(ResizeDirection, CursorIcon, Rect); 8] {
    use {CursorIcon as C, ResizeDirection as D};
    let (l, t, r, b) = (full.left(), full.top(), full.right(), full.bottom());
    let e = EDGE;
    let k = 3.0 * EDGE;
    let rect = |x0, y0, x1, y1| Rect::from_min_max(pos2(x0, y0), pos2(x1, y1));
    [
        (D::North, C::ResizeNorth, rect(l + k, t, r - k, t + e)),
        (D::South, C::ResizeSouth, rect(l + k, b - e, r - k, b)),
        (D::West, C::ResizeWest, rect(l, t + k, l + e, b - k)),
        (D::East, C::ResizeEast, rect(r - e, t + k, r, b - k)),
        (D::NorthWest, C::ResizeNorthWest, rect(l, t, l + k, t + e)),
        (D::NorthEast, C::ResizeNorthEast, rect(r - k, t, r, t + e)),
        (D::SouthWest, C::ResizeSouthWest, rect(l, b - e, l + k, b)),
        (D::SouthEast, C::ResizeSouthEast, rect(r - k, b - e, r, b)),
    ]
}
