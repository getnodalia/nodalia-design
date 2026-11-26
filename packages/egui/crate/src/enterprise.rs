//! The pieces shared by the Nodalia enterprise apps (Ágora and Hito today; customer management,
//! resource management and finance later), so the suite reads as one family: the same window frame,
//! the same top bar, the same workspace switcher, sidebar rows and account card, the same sign-in
//! card, the same connection pill and the same integration cards.
//!
//! The specification is `docs/suites/enterprise.md`. The module only draws: each app passes its
//! labels already translated and acts on the responses by running its own commands. Colours come
//! from the app's [`Palette`](crate::Palette) through [`Look`], so a Nodalia theme and the app's
//! accent are kept.
//!
//! An enterprise app is a client of a server the company runs. The window is always the same shape:
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │ ≡  ◆ App      [        search        ]      (page action) ● Connected ●●● │  top bar 44
//! ├───────────────┬───────────────────────────────────────┬────────────────┤
//! │ ▣ Workspace ⌄ │                                       │                │
//! │   Overview    │               page (island)           │ detail (island)│
//! │   Section  +  │                                       │  (optional)    │
//! │   rows …      │                                       │                │
//! │ ● Account     │                                       │                │
//! └───────────────┴───────────────────────────────────────┴────────────────┘
//! ```
//!
//! ```no_run
//! # fn draw(ui: &mut egui::Ui) {
//! use nodalia_theme::creator::Look;
//! use nodalia_theme::enterprise::{self as ent, Panes};
//! use nodalia_theme::{Accent, Mode};
//!
//! let look = Look::new(Mode::Light.palette_with(Accent::Hito));
//! let panes = Panes::new(ui.max_rect(), Some(ent::layout::DETAIL));
//! ent::island(ui, &look, panes.side, "sidebar", |ui| {
//!     let me = ent::Account { seed: 1, name: "Ana", detail: "@ana", online: Some(true) };
//!     ent::sidebar(ui, &look, &me, |ui| {
//!         ent::workspace_switcher(ui, &look, "Nodalia Studio", "Workspaces", |_ui| {});
//!     });
//! });
//! # }
//! ```

use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Frame, Id, Layout, Margin, Rect,
    Response, RichText, Role, Sense, Stroke, StrokeKind, Ui, UiBuilder, WidgetInfo,
};

use crate::creator::{self as cr, Look};
use crate::for_egui::color;
use crate::{font_size, icons};

pub mod chrome;
pub mod integrations;

/// The family line the About page shows ("Part of Nodalia · Enterprise"), before translation.
pub const SUITE: &str = "Enterprise";

/// The suite's measures, in points. The bar, the gaps, the radii and the control heights are the
/// creator suite's, so an island looks the same in every Nodalia app.
pub mod layout {
    pub use crate::creator::layout::{
        APP_BAR, BUTTON_HEIGHT, BUTTON_MIN_WIDTH, CLOSE, CONTROL_HEIGHT, CONTROL_RADIUS,
        FIELD_RADIUS, GAP, ISLAND_RADIUS,
    };

    /// The left island: workspace, navigation and account.
    pub const SIDEBAR: f32 = 260.0;
    /// The optional right island: a task's details, a conversation's details, a thread.
    pub const DETAIL: f32 = 340.0;
    /// The narrowest the page may get before the detail island gives way.
    pub const MIN_PAGE: f32 = 360.0;
    /// The account card at the foot of the sidebar.
    pub const ACCOUNT: f32 = 52.0;
    /// The workspace switcher at the head of the sidebar.
    pub const SWITCHER: f32 = 40.0;
    /// A sidebar row (a channel, a project, Overview).
    pub const ROW: f32 = 30.0;
    /// The header of a detail island.
    pub const PANEL_HEADER: f32 = 44.0;
    /// The sign-in card's width.
    pub const SIGN_IN: f32 = 380.0;
    /// A round icon button in a bar or a header.
    pub const ICON_BUTTON: f32 = 28.0;
}

// --- The window ----------------------------------------------------------------------------------

/// Where the window's parts go for a given size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panes {
    /// The top bar.
    pub bar: Rect,
    /// The left island.
    pub side: Rect,
    /// The page's island. When the detail island was asked for and does not fit
    /// ([`Panes::squeezed`]), the app draws the detail here instead of the page.
    pub center: Rect,
    /// The right island, when it was asked for and fits.
    pub detail: Option<Rect>,
    /// The detail island was asked for but the window is too narrow for three islands.
    pub squeezed: bool,
}

impl Panes {
    /// Split `full` (the window's whole area) into the bar and the islands. `detail` is the width
    /// of the right island when the app wants one, such as [`layout::DETAIL`].
    pub fn new(full: Rect, detail: Option<f32>) -> Panes {
        let gap = layout::GAP;
        let bar = full.with_max_y(full.top() + layout::APP_BAR);
        let body = Rect::from_min_max(
            pos2(full.left() + gap, bar.bottom()),
            pos2(
                (full.right() - gap).max(full.left() + gap),
                (full.bottom() - gap).max(bar.bottom()),
            ),
        );
        let side_right = (body.left() + layout::SIDEBAR).min(body.right());
        let side = Rect::from_min_max(body.min, pos2(side_right, body.bottom()));
        let wanted = detail.map(|w| w.max(0.0));
        let fits = wanted.is_some_and(|w| body.width() > layout::SIDEBAR + w + layout::MIN_PAGE);
        let detail_rect = match wanted {
            Some(w) if fits => Some(Rect::from_min_max(
                pos2(body.right() - w, body.top()),
                body.max,
            )),
            _ => None,
        };
        let center_right = detail_rect.map_or(body.right(), |r| r.left() - gap);
        let center = Rect::from_min_max(
            pos2((side.right() + gap).min(center_right), body.top()),
            pos2(center_right, body.bottom()),
        );
        Panes {
            bar,
            side,
            center,
            detail: detail_rect,
            squeezed: wanted.is_some() && !fits,
        }
    }
}

/// Paint the window's surface behind everything and return its area.
pub fn window_surface(ui: &mut Ui, look: &Look) -> Rect {
    let full = ui.max_rect();
    ui.painter()
        .rect_filled(full, 0.0, color(look.palette.surface_window));
    full
}

/// An island at `rect`: the panel surface with the suite's corners, filled to the rectangle, with
/// `add` drawn inside. `id` keeps its widgets' ids apart from the other islands'.
pub fn island(ui: &mut Ui, look: &Look, rect: Rect, id: &str, add: impl FnOnce(&mut Ui)) {
    ui.scope_builder(UiBuilder::new().max_rect(rect).id_salt(id), |ui| {
        cr::island(look).show(ui, |ui| {
            ui.set_min_size(rect.size());
            ui.set_max_size(rect.size());
            add(ui);
        });
    });
}

// --- The top bar ---------------------------------------------------------------------------------

/// Draw the top bar's frame: the drag area, the room for the window controls at the desktop's side
/// (and for macOS's lights), and the controls themselves. `add` lays the bar's content out left to
/// right in what is left, vertically centred.
pub fn top_bar(
    ui: &mut Ui,
    look: &Look,
    mode: crate::Mode,
    ndl_controls: bool,
    bar: Rect,
    labels: crate::window_controls::Labels<'_>,
    add: impl FnOnce(&mut Ui),
) {
    if chrome::drags_window(ndl_controls) {
        chrome::drag_area(ui, bar);
    }
    // Controls at the left (the desktop's choice on Linux) take the bar's first stretch.
    let controls_left = ndl_controls && chrome::side() == crate::titlebar::Side::Left;
    let right_reserved = if ndl_controls && !controls_left {
        chrome::CONTROLS_WIDTH
    } else {
        layout::GAP
    };
    let left_reserved = if controls_left {
        chrome::CONTROLS_WIDTH + layout::GAP
    } else {
        layout::GAP + chrome::leading_inset()
    };
    let inner = Rect::from_min_max(
        pos2(bar.left() + left_reserved, bar.top()),
        pos2(
            (bar.right() - right_reserved).max(bar.left() + left_reserved),
            bar.bottom(),
        ),
    );
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::left_to_right(Align::Center)),
        add,
    );
    if ndl_controls {
        chrome::controls(ui, bar, &look.palette, mode, labels);
    }
}

/// The hamburger button at the left of the bar, with `menu` as the list it opens. By the suite's
/// order, the app's own entries come first, then Preferences, Keyboard shortcuts, About and Check
/// for updates, then, apart, signing out.
pub fn main_menu(ui: &mut Ui, look: &Look, label: &str, menu: impl FnOnce(&mut Ui)) -> Response {
    let button = icon_button(ui, look, "menu", label, false);
    egui::Popup::menu(&button).show(|ui| {
        ui.set_min_width(220.0);
        menu(ui);
    });
    button
}

/// The app's icon drawn from a texture, with the corners every Nodalia app icon gets.
pub fn logo(ui: &mut Ui, texture: &egui::TextureHandle, size: f32) {
    ui.add(
        egui::Image::new(texture)
            .fit_to_exact_size(vec2(size, size))
            .corner_radius(CornerRadius::same((size * 0.22) as u8)),
    );
}

/// The app's brand at the left of the bar: its icon and its name.
pub fn brand(ui: &mut Ui, look: &Look, texture: &egui::TextureHandle, name: &str) {
    ui.add_space(4.0);
    logo(ui, texture, 22.0);
    ui.label(
        RichText::new(name)
            .strong()
            .size(font_size::LARGE)
            .color(color(look.palette.text_primary)),
    );
}

/// The bar's search field, centred in what is left of the row. `trailing` is how much room the
/// content at the right takes (the pill and the page action), so the field stays centred in the
/// window rather than in the free space.
pub fn bar_search(
    ui: &mut Ui,
    look: &Look,
    text: &mut String,
    hint: &str,
    trailing: f32,
) -> Response {
    let rest = ui.available_width();
    let width = (rest - trailing).clamp(160.0, 420.0);
    ui.add_space(((rest - width) / 2.0 - trailing / 2.0).max(8.0));
    search_field(ui, look, text, hint, width)
}

/// Lay out `add` from the right edge, on the current row. `with_layout` would claim the whole
/// height left in a horizontal row and push the row's height to it.
pub fn trailing<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let row = if ui.layout().is_horizontal() {
        ui.min_rect().height()
    } else {
        0.0
    };
    let height = row.max(ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), height),
        Layout::right_to_left(Align::Center),
        add,
    )
    .inner
}

// --- Small parts ---------------------------------------------------------------------------------

/// Paint an NDL icon in a square of `size` points.
pub fn icon(ui: &mut Ui, name: &str, size: f32, ink: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if let Some(i) = icons::get(name) {
        icons::paint(ui.painter(), rect, i, icons::Style::bold(ink));
    }
    response
}

/// A round, borderless icon button with an accessible label (also its hover text).
pub fn icon_button(ui: &mut Ui, look: &Look, name: &str, label: &str, active: bool) -> Response {
    let size = layout::ICON_BUTTON;
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let p = &look.palette;
    let fill = if active {
        color(p.accent_subtle)
    } else if response.hovered() {
        color(p.control_bg_hover)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(layout::CONTROL_RADIUS), fill);
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            CornerRadius::same(12),
            Stroke::new(2.0, color(p.border_focus)),
            StrokeKind::Outside,
        );
    }
    let ink = if active {
        color(p.accent_text)
    } else {
        color(p.text_secondary)
    };
    if let Some(i) = icons::get(name) {
        icons::paint(ui.painter(), rect.shrink(6.0), i, icons::Style::bold(ink));
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
    response.on_hover_text(label)
}

/// Colours for avatars, from the palette's status and accent colours so they follow the theme.
fn avatar_fill(look: &Look, seed: i64) -> Color32 {
    let p = &look.palette;
    let choices = [
        p.accent_default,
        p.status_info_fg,
        p.status_success_fg,
        p.status_warning_fg,
        p.status_danger_fg,
    ];
    let i = usize::try_from(seed.rem_euclid(choices.len() as i64)).unwrap_or(0);
    choices.get(i).map_or(Color32::GRAY, |c| color(*c))
}

/// A round avatar with the person's initial. `seed` picks its colour (use the person's id) and
/// `online` adds the presence dot: filled when connected, a ring when not.
pub fn avatar(ui: &mut Ui, look: &Look, seed: i64, name: &str, size: f32, online: Option<bool>) {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Image, true, name));
    let painter = ui.painter();
    painter.circle_filled(rect.center(), size / 2.0, avatar_fill(look, seed));
    let initial: String = name
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default();
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        initial,
        FontId::proportional(size * 0.45),
        color(look.palette.text_on_accent),
    );
    if let Some(online) = online {
        let r = (size * 0.16).max(3.5);
        let c = rect.right_bottom() - vec2(r, r);
        painter.circle_filled(c, r + 1.5, color(look.palette.surface_panel));
        if online {
            painter.circle_filled(c, r, color(look.palette.status_success_fg));
        } else {
            painter.circle_stroke(
                c,
                r - 0.5,
                Stroke::new(1.5, color(look.palette.text_tertiary)),
            );
        }
    }
}

/// A pill with a count: unread items (`strong` false) or mentions and things that need the person
/// (`strong` true, in the accent).
pub fn badge(ui: &mut Ui, look: &Look, count: u32, strong: bool) {
    let text = if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    };
    let p = &look.palette;
    let (fill, ink) = if strong {
        (color(p.accent_default), color(p.text_on_accent))
    } else {
        (color(p.control_bg_hover), color(p.text_secondary))
    };
    Frame::NONE
        .fill(fill)
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(font_size::CAPTION)
                    .color(ink)
                    .strong(),
            );
        });
}

/// A small rounded label in a colour pair, painted at its own height whatever the row's.
pub fn chip(ui: &mut Ui, text: &str, ink: Color32, fill: Color32) -> Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        FontId::proportional(font_size::CAPTION),
        ink,
    );
    let size = galley.size() + vec2(14.0, 4.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same((size.y / 2.0) as u8), fill);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, ink);
    response
}

/// A thin rounded progress bar, `width` wide (the available width when 0), with `label` beside it
/// when given. `fraction` is 0 to 1; a complete bar is drawn in the success colour.
pub fn progress_bar(
    ui: &mut Ui,
    look: &Look,
    fraction: f32,
    complete: bool,
    width: f32,
    label: Option<&str>,
) {
    let p = &look.palette;
    let width = if width <= 0.0 {
        ui.available_width() - if label.is_some() { 44.0 } else { 0.0 }
    } else {
        width
    };
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(width.max(24.0), 8.0), Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(4), color(p.control_bg_hover));
        let fraction = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if fraction > 0.0 {
            let mut done = rect;
            done.set_width((rect.width() * fraction).max(8.0));
            let fill = if complete {
                color(p.status_success_fg)
            } else {
                color(p.accent_default)
            };
            painter.rect_filled(done, CornerRadius::same(4), fill);
        }
        if let Some(label) = label {
            ui.label(
                RichText::new(label)
                    .size(font_size::SMALL)
                    .color(color(p.text_secondary)),
            );
        }
    });
}

/// The bar's search field: NDL's field frame with a search glyph, the text centred on the control's
/// height (egui's tall interact size would otherwise push it to the top).
pub fn search_field(
    ui: &mut Ui,
    look: &Look,
    text: &mut String,
    hint: &str,
    width: f32,
) -> Response {
    let p = &look.palette;
    cr::field_frame(look)
        .inner_margin(Margin::symmetric(8, 0))
        .show(ui, |ui| {
            ui.set_width(width - 18.0);
            ui.horizontal(|ui| {
                let (glyph, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                if let Some(i) = icons::get("search") {
                    icons::paint(
                        ui.painter(),
                        glyph,
                        i,
                        icons::Style::regular(color(p.text_secondary)),
                    );
                }
                ui.add(
                    egui::TextEdit::singleline(text)
                        .frame(Frame::NONE)
                        .align(Align2::LEFT_CENTER)
                        .hint_text(hint)
                        .desired_width(ui.available_width()),
                )
            })
            .inner
        })
        .inner
}

/// The action that destroys something for good (delete a workspace): filled in the danger colour
/// with white ink, like the primary button in shape. Keep it last in the row, enabled only once the
/// person has confirmed.
pub fn danger_button(ui: &mut Ui, look: &Look, text: &str, enabled: bool) -> Response {
    let p = &look.palette;
    let ink = if enabled {
        Color32::WHITE
    } else {
        color(p.text_disabled)
    };
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(13.0), ink);
    let width = (galley.size().x + 28.0).max(layout::BUTTON_MIN_WIDTH);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, layout::BUTTON_HEIGHT), sense);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled && ui.is_enabled(), text));
    let fill = match (enabled, response.hovered()) {
        (false, _) => color(p.control_bg_disabled),
        (true, true) => color(p.status_danger_fg).gamma_multiply(0.85),
        (true, false) => color(p.status_danger_fg),
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(layout::CONTROL_RADIUS), fill);
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, ink);
    response
}

/// How a pill or a status line reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Success,
    Info,
    Warning,
    Danger,
}

impl Tone {
    /// The tone's text and background colours.
    pub fn colors(self, look: &Look) -> (Color32, Color32) {
        let p = &look.palette;
        let (fg, bg) = match self {
            Tone::Success => (p.status_success_fg, p.status_success_bg),
            Tone::Info => (p.status_info_fg, p.status_info_bg),
            Tone::Warning => (p.status_warning_fg, p.status_warning_bg),
            Tone::Danger => (p.status_danger_fg, p.status_danger_bg),
        };
        (color(fg), color(bg))
    }
}

/// A pill with a dot and a short text, in a status tone. It is the connection pill of the bar
/// ("Connected", "Connecting…", "Disconnected. Retrying in 5 s.") and any small status.
pub fn status_pill(ui: &mut Ui, look: &Look, tone: Tone, text: &str) -> Response {
    let (fg, bg) = tone.colors(look);
    let response = Frame::NONE
        .fill(bg)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(10, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                ui.painter().circle_filled(dot.center(), 4.0, fg);
                ui.label(RichText::new(text).size(font_size::SMALL).color(fg));
            });
        })
        .response;
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, text));
    response
}

/// A line of text on a tinted background, for a notice or an error under a field.
pub fn status_line(ui: &mut Ui, look: &Look, text: &str, danger: bool) {
    let tone = if danger { Tone::Danger } else { Tone::Info };
    let (fg, bg) = tone.colors(look);
    Frame::NONE
        .fill(bg)
        .corner_radius(8)
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(text).size(font_size::SMALL).color(fg));
        });
}

/// A centred hint in an empty area, with a large glyph over it.
pub fn empty_state(ui: &mut Ui, look: &Look, glyph: &str, text: &str) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        icon(ui, glyph, 40.0, color(look.palette.text_tertiary));
        ui.add_space(10.0);
        ui.label(RichText::new(text).color(color(look.palette.text_secondary)));
    });
}

/// Short notices at the bottom of the window, newest at the bottom. `texts` runs from the oldest.
pub fn toasts<'a>(ui: &mut Ui, look: &Look, texts: impl DoubleEndedIterator<Item = &'a str>) {
    let p = &look.palette;
    let full = ui.max_rect();
    let mut y = full.bottom() - 24.0;
    for text in texts.rev() {
        let galley = ui.painter().layout(
            text.to_owned(),
            FontId::proportional(font_size::BODY),
            color(p.text_inverse),
            340.0,
        );
        let size = galley.size() + vec2(28.0, 18.0);
        let rect = Rect::from_min_size(pos2(full.center().x - size.x / 2.0, y - size.y), size);
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            Id::NULL.with("nodalia_enterprise_toasts"),
        ));
        painter.rect_filled(rect, CornerRadius::same(12), color(p.surface_inverse));
        painter.galley(rect.min + vec2(14.0, 9.0), galley, color(p.text_inverse));
        y = rect.top() - 8.0;
    }
}

// --- The sidebar ---------------------------------------------------------------------------------

/// Who is signed in, for the account card at the foot of the sidebar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Account<'a> {
    /// Picks the avatar's colour; the person's id.
    pub seed: i64,
    /// The display name.
    pub name: &'a str,
    /// The second line: the handle, and the presence when the app has it ("@ana · Online").
    pub detail: &'a str,
    /// The presence dot on the avatar.
    pub online: Option<bool>,
}

/// The sidebar's content, inside its island: `top` above (the workspace switcher, the navigation,
/// the lists) and the account card at the foot, under a hairline.
pub fn sidebar(ui: &mut Ui, look: &Look, account: &Account<'_>, top: impl FnOnce(&mut Ui)) {
    let p = &look.palette;
    let full = ui.max_rect();
    let footer_top = (full.bottom() - layout::ACCOUNT).max(full.top());
    let upper = Rect::from_min_max(full.min, pos2(full.right(), footer_top));
    let footer = Rect::from_min_max(pos2(full.left(), footer_top), full.max);

    ui.scope_builder(
        UiBuilder::new().max_rect(upper.shrink2(vec2(8.0, 8.0))),
        top,
    );

    ui.painter().line_segment(
        [
            pos2(footer.left() + 12.0, footer.top()),
            pos2(footer.right() - 12.0, footer.top()),
        ],
        Stroke::new(1.0, color(p.border_subtle)),
    );
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(footer.shrink2(vec2(12.0, 8.0)))
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            avatar(ui, look, account.seed, account.name, 30.0, account.online);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(account.name)
                            .strong()
                            .color(color(p.text_primary)),
                    )
                    .truncate(),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(account.detail)
                            .size(font_size::CAPTION)
                            .color(color(p.text_secondary)),
                    )
                    .truncate(),
                );
            });
        },
    );
}

/// The workspace switcher at the head of the sidebar: the workspace's initial on an accent tile,
/// its name and a chevron. Clicking it opens `menu` (the other workspaces, then create and join).
/// `label` names the button for screen readers ("Workspaces").
pub fn workspace_switcher(
    ui: &mut Ui,
    look: &Look,
    name: &str,
    label: &str,
    menu: impl FnOnce(&mut Ui),
) -> Response {
    let p = &look.palette;
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), layout::SWITCHER), Sense::click());
    let fill = if response.hovered() {
        color(p.control_bg_hover)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(layout::CONTROL_RADIUS), fill);
    let tile = Rect::from_min_size(rect.min + vec2(6.0, 6.0), vec2(28.0, 28.0));
    ui.painter()
        .rect_filled(tile, CornerRadius::same(8), color(p.accent_default));
    let initial: String = name
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default();
    ui.painter().text(
        tile.center(),
        Align2::CENTER_CENTER,
        initial,
        FontId::proportional(15.0),
        color(p.text_on_accent),
    );
    let mut job = egui::text::LayoutJob::simple_singleline(
        name.to_owned(),
        FontId::proportional(font_size::LARGE),
        color(p.text_primary),
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(
        (rect.right() - 36.0 - (tile.right() + 10.0)).max(20.0),
    );
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(
        pos2(tile.right() + 10.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color(p.text_primary),
    );
    if let Some(chevron) = icons::get("chevron-down") {
        let r =
            Rect::from_center_size(pos2(rect.right() - 16.0, rect.center().y), vec2(14.0, 14.0));
        icons::paint(
            ui.painter(),
            r,
            chevron,
            icons::Style::bold(color(p.text_secondary)),
        );
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
    egui::Popup::menu(&response).show(|ui| {
        ui.set_min_width(rect.width() - 8.0);
        menu(ui);
    });
    response
}

/// A section's heading in the sidebar ("Channels", "Projects") with, at the right, a plus button
/// when the person may add to it. True when the plus was clicked.
pub fn section_heading(ui: &mut Ui, look: &Look, title: &str, add: Option<&str>) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        ui.label(
            RichText::new(title)
                .size(font_size::SMALL)
                .strong()
                .color(color(look.palette.text_secondary)),
        );
        if let Some(label) = add {
            trailing(ui, |ui| {
                clicked = icon_button(ui, look, "plus", label, false).clicked();
            });
        }
    });
    clicked
}

/// What a sidebar row starts with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lead<'a> {
    None,
    /// An NDL icon.
    Icon(&'a str),
    /// An avatar (a direct message): the person's id, name and presence.
    Avatar {
        seed: i64,
        name: &'a str,
        online: Option<bool>,
    },
}

/// What a sidebar row ends with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Count {
    None,
    /// Unread or open items: a quiet pill.
    Quiet(u32),
    /// Mentions or things that need the person: a pill in the accent.
    Strong(u32),
}

/// A row of the sidebar: Overview, a channel, a direct message, a list. The selected row is tinted
/// with the accent; a row with something new is in the strong face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row<'a> {
    pub lead: Lead<'a>,
    pub text: &'a str,
    pub selected: bool,
    /// Something new here: the name in the strong face and primary ink.
    pub bold: bool,
    /// Muted (a channel the person silenced, an archived project).
    pub muted: bool,
    pub count: Count,
}

impl<'a> Row<'a> {
    pub fn new(text: &'a str) -> Self {
        Row {
            lead: Lead::None,
            text,
            selected: false,
            bold: false,
            muted: false,
            count: Count::None,
        }
    }

    pub fn icon(mut self, name: &'a str) -> Self {
        self.lead = Lead::Icon(name);
        self
    }

    pub fn avatar(mut self, seed: i64, online: Option<bool>) -> Self {
        self.lead = Lead::Avatar {
            seed,
            name: self.text,
            online,
        };
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    pub fn count(mut self, count: Count) -> Self {
        self.count = count;
        self
    }

    /// Draw the row across the available width.
    pub fn show(self, ui: &mut Ui, look: &Look) -> Response {
        let p = &look.palette;
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), layout::ROW), Sense::click());
        let radius = CornerRadius::same(layout::CONTROL_RADIUS);
        let fill = if self.selected {
            color(p.accent_subtle)
        } else if response.hovered() {
            color(p.control_bg_hover)
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, radius, fill);
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect,
                radius,
                Stroke::new(2.0, color(p.border_focus)),
                StrokeKind::Inside,
            );
        }
        let strong = self.bold && !self.selected;
        let ink = if self.selected {
            color(p.accent_text)
        } else if self.muted {
            color(p.text_tertiary)
        } else if strong {
            color(p.text_primary)
        } else {
            color(p.text_secondary)
        };
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(rect.shrink2(vec2(8.0, 0.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );
        match self.lead {
            Lead::None => {}
            Lead::Icon(name) => {
                icon(&mut child, name, 16.0, ink);
                child.add_space(6.0);
            }
            Lead::Avatar { seed, name, online } => {
                avatar(&mut child, look, seed, name, 20.0, online);
                child.add_space(6.0);
            }
        }
        let mut text = RichText::new(self.text).color(ink);
        if strong || self.selected {
            text = text.strong();
        }
        child.add(egui::Label::new(text).truncate().selectable(false));
        child.with_layout(Layout::right_to_left(Align::Center), |ui| {
            match self.count {
                Count::None => {}
                Count::Quiet(n) if n > 0 && !self.selected => badge(ui, look, n, false),
                Count::Quiet(_) => {}
                Count::Strong(n) if n > 0 => badge(ui, look, n, true),
                Count::Strong(_) => {}
            }
        });
        response.widget_info(|| WidgetInfo::selected(Role::Button, true, self.selected, self.text));
        response
    }
}

/// The header of a detail island: a title, a hairline under it and, when `closable`, the round
/// close button at the right. True when the close button was clicked.
pub fn panel_header(ui: &mut Ui, look: &Look, title: &str, closable: bool) -> bool {
    let mut closed = false;
    ui.horizontal(|ui| {
        ui.set_height(layout::PANEL_HEADER);
        ui.add_space(14.0);
        ui.label(
            RichText::new(title)
                .size(font_size::LARGE)
                .strong()
                .color(color(look.palette.text_primary)),
        );
        if closable {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                let (rect, _) =
                    ui.allocate_exact_size(vec2(layout::CLOSE, layout::CLOSE), Sense::hover());
                closed = cr::close_button(
                    ui,
                    look,
                    rect,
                    Id::NULL.with(("nodalia_close_panel", title)),
                );
            });
        }
    });
    let y = ui.min_rect().bottom();
    ui.painter().line_segment(
        [
            pos2(ui.max_rect().left(), y),
            pos2(ui.max_rect().right(), y),
        ],
        Stroke::new(1.0, color(look.palette.border_subtle)),
    );
    closed
}

// --- Sign-in -------------------------------------------------------------------------------------

/// The sign-in page's body: `add` draws the card, centred in `body`, which is the window under
/// the top bar. `card_height` is the card's usual height, to centre it vertically; a short window
/// scrolls.
pub fn sign_in_page(ui: &mut Ui, body: Rect, card_height: f32, add: impl FnOnce(&mut Ui)) {
    ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let side = ((ui.available_width() - layout::SIGN_IN) / 2.0).max(8.0);
                ui.add_space(((body.height() - card_height) / 2.0).max(8.0));
                ui.horizontal(|ui| {
                    ui.add_space(side);
                    ui.vertical(add);
                });
                ui.add_space(16.0);
            });
    });
}

/// The sign-in card: an island with a 24-point margin, [`layout::SIGN_IN`] wide.
pub fn sign_in_card<R>(ui: &mut Ui, look: &Look, add: impl FnOnce(&mut Ui) -> R) -> R {
    cr::island(look)
        .inner_margin(24)
        .show(ui, |ui| {
            ui.set_width(layout::SIGN_IN - 48.0);
            add(ui)
        })
        .inner
}

/// The head of the sign-in card: the app's icon, the title and what the app is, centred.
pub fn sign_in_head(
    ui: &mut Ui,
    look: &Look,
    texture: &egui::TextureHandle,
    title: &str,
    tagline: &str,
) {
    let p = &look.palette;
    ui.vertical_centered(|ui| {
        logo(ui, texture, 72.0);
        ui.add_space(8.0);
        ui.label(
            RichText::new(title)
                .size(font_size::TITLE)
                .strong()
                .color(color(p.text_primary)),
        );
        ui.label(
            RichText::new(tagline)
                .size(font_size::SMALL)
                .color(color(p.text_secondary)),
        );
    });
    ui.add_space(16.0);
}

/// The small label over a form field.
pub fn field_label(ui: &mut Ui, look: &Look, text: &str) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(text)
            .size(font_size::SMALL)
            .color(color(look.palette.text_secondary)),
    );
}

/// A one-line text field in the suite's frame, the text centred on the control's height.
/// `password` hides what is typed.
pub fn field(ui: &mut Ui, look: &Look, text: &mut String, hint: &str, password: bool) -> Response {
    cr::field_frame(look)
        .inner_margin(Margin::symmetric(8, 0))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .frame(Frame::NONE)
                    .align(Align2::LEFT_CENTER)
                    .hint_text(hint)
                    .password(password)
                    .desired_width(ui.available_width()),
            )
        })
        .inner
}

// --- The language picker -------------------------------------------------------------------------

/// A dropdown for choosing one of many options, such as the interface language: the current choice
/// in a field-shaped pill, and a list under it with the options in rows, a check at the right of
/// the one in use. A list of more than [`PICKER_SEARCH_FROM`] items starts with a search field, and
/// the list scrolls. Returns the index into `items` picked this frame.
pub fn picker(
    ui: &mut Ui,
    look: &Look,
    id: &str,
    current: &str,
    items: &[&str],
    selected: Option<usize>,
    width: f32,
) -> Option<usize> {
    let p = &look.palette;
    let popup_id = Id::NULL.with(("nodalia_enterprise_picker", id));
    let (rect, resp) = ui.allocate_exact_size(vec2(width, layout::CONTROL_HEIGHT), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    resp.widget_info(|| WidgetInfo::labeled(Role::ComboBox, true, current));
    let open = egui::Popup::is_id_open(ui.ctx(), popup_id);
    let fill = if open || resp.hovered() {
        color(p.control_bg_hover)
    } else {
        color(p.control_field_bg)
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(layout::FIELD_RADIUS),
        fill,
        Stroke::new(1.0, color(p.border_default)),
        StrokeKind::Inside,
    );
    if let Some(chevron) = icons::get("chevron-down") {
        icons::paint(
            ui.painter(),
            Rect::from_center_size(pos2(rect.right() - 16.0, rect.center().y), vec2(12.0, 12.0)),
            chevron,
            icons::Style::bold(color(p.text_secondary)),
        );
    }
    let mut job = egui::text::LayoutJob::simple_singleline(
        current.to_owned(),
        FontId::proportional(font_size::BODY),
        color(p.text_primary),
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(rect.width() - 40.0);
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(
        pos2(rect.left() + 10.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color(p.text_primary),
    );

    let query_id = popup_id.with("query");
    let mut picked = None;
    egui::Popup::menu(&resp).id(popup_id).show(|ui| {
        ui.set_min_width(width.max(220.0));
        let mut query = ui
            .data_mut(|d| d.get_temp::<String>(query_id))
            .unwrap_or_default();
        if items.len() > PICKER_SEARCH_FROM {
            search_field(ui, look, &mut query, "", width.max(220.0) - 12.0);
            ui.add_space(4.0);
        }
        let needle = query.trim().to_lowercase();
        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                for (i, name) in items.iter().enumerate() {
                    if !needle.is_empty() && !name.to_lowercase().contains(&needle) {
                        continue;
                    }
                    let checked = selected == Some(i);
                    let (row, response) = ui.allocate_exact_size(
                        vec2(ui.available_width(), layout::ROW),
                        Sense::click(),
                    );
                    if response.hovered() {
                        ui.painter().rect_filled(
                            row,
                            CornerRadius::same(layout::CONTROL_RADIUS),
                            color(p.control_bg_hover),
                        );
                    }
                    let ink = if checked {
                        color(p.accent_text)
                    } else {
                        color(p.text_primary)
                    };
                    let mut text = RichText::new(*name).color(ink);
                    if checked {
                        text = text.strong();
                    }
                    let galley = egui::WidgetText::from(text).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Truncate),
                        row.width() - 44.0,
                        egui::TextStyle::Body,
                    );
                    ui.painter().galley(
                        pos2(row.left() + 10.0, row.center().y - galley.size().y / 2.0),
                        galley,
                        ink,
                    );
                    if checked {
                        if let Some(check) = icons::get("check") {
                            icons::paint(
                                ui.painter(),
                                Rect::from_center_size(
                                    pos2(row.right() - 16.0, row.center().y),
                                    vec2(14.0, 14.0),
                                ),
                                check,
                                icons::Style::bold(color(p.accent_text)),
                            );
                        }
                    }
                    response
                        .widget_info(|| WidgetInfo::selected(Role::MenuItem, true, checked, name));
                    if response.clicked() {
                        picked = Some(i);
                        ui.close();
                    }
                }
            });
        ui.data_mut(|d| d.insert_temp(query_id, query));
    });
    if picked.is_some() {
        ui.data_mut(|d| d.remove::<String>(query_id));
    }
    picked
}

/// A [`picker`] list with more items than this starts with a search field.
pub const PICKER_SEARCH_FROM: usize = 8;
