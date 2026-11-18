//! The About window every Nodalia egui app shows: the same heading with its round close button,
//! the same tab row (About, Contributors, Models) and the same About page (the app's mark, name,
//! version, a line on what it is, the suite it belongs to, the links and the licence line).
//!
//! The specification is `docs/suites/about.md`. The module only draws: the app owns the window
//! (its modal, its drag, its placement), supplies its mark as a painter callback and its texts
//! already translated, fills the Contributors and Models tabs with its own lists, and opens the
//! address of the link the page reports as clicked.
//!
//! ```no_run
//! # fn draw(ui: &mut egui::Ui) {
//! use nodalia_theme::about::{self, Info, Link};
//! use nodalia_theme::creator::Look;
//! use nodalia_theme::{Accent, Mode};
//!
//! let look = Look::new(Mode::Light.palette_with(Accent::Pulso));
//! let links = [Link { label: "Website", url: "https://getnodalia.com/pulso" }];
//! let info = Info {
//!     name: "Pulso",
//!     version: "Version 0.1.1",
//!     tagline: "Motion graphics and visual effects.",
//!     suite: "Part of Nodalia · Creator",
//!     links: &links,
//!     footer: "MIT OR Apache-2.0",
//! };
//! if about::heading(ui, &look, "About Pulso", egui::Id::NULL.with("about.close")) {
//!     // close the window
//! }
//! let tab = about::tabs(ui, &look, &["About", "Contributors", "Models"], 0);
//! if tab == 0 {
//!     if let Some(i) = about::page(ui, &look, &info, |ui, rect| { /* paint the mark in rect */ }) {
//!         // open links[i].url
//!     }
//! }
//! # }
//! ```

use egui::{
    pos2, vec2, Align2, Color32, FontId, Id, Rect, Response, Role, Sense, Stroke, Ui, WidgetInfo,
};

use crate::creator::{self, layout, Look};
use crate::for_egui::color;

/// The About window's measures, in points.
pub mod measures {
    /// The window's width on the About tab; Contributors and Models use [`WIDE`].
    pub const WIDTH: f32 = 460.0;
    /// The window's width on the tabs that hold a list.
    pub const WIDE: f32 = 820.0;
    /// The app's mark.
    pub const MARK: f32 = 96.0;
    /// Between the links of a row.
    pub const LINK_GAP: f32 = 18.0;
}

/// A link on the About page.
#[derive(Clone, Debug, PartialEq)]
pub struct Link<'a> {
    pub label: &'a str,
    /// Shown on hover; the app opens it.
    pub url: &'a str,
}

/// What the About page says, already translated.
#[derive(Clone, Debug, PartialEq)]
pub struct Info<'a> {
    pub name: &'a str,
    /// The whole line, such as "Version 0.1.1".
    pub version: &'a str,
    /// What the app is, one or two sentences.
    pub tagline: &'a str,
    /// The family it belongs to, such as "Part of Nodalia · Creator".
    pub suite: &'a str,
    /// The website, the source, the issue tracker.
    pub links: &'a [Link<'a>],
    /// The licence line, small, at the foot.
    pub footer: &'a str,
}

fn font(look: &Look, size: f32, strong: bool) -> FontId {
    FontId::new(
        size,
        if strong {
            look.strong.clone()
        } else {
            look.medium.clone()
        },
    )
}

/// The window's heading: the title in the strong face at the left and the round close button at
/// the right, at the same distance from the edge as the title. Returns whether the button was
/// clicked.
pub fn heading(ui: &mut Ui, look: &Look, title: &str, close_id: Id) -> bool {
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), layout::CLOSE), Sense::hover());
    ui.painter().text(
        pos2(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        title,
        font(look, 15.0, true),
        color(look.palette.text_primary),
    );
    let button = Rect::from_min_size(
        pos2(rect.right() - layout::CLOSE, rect.top()),
        vec2(layout::CLOSE, layout::CLOSE),
    );
    creator::close_button(ui, look, button, close_id)
}

/// The tab row under the heading: one pill per label, the selected one grey. Returns the index
/// now selected.
pub fn tabs(ui: &mut Ui, look: &Look, labels: &[&str], selected: usize) -> usize {
    let mut now = selected;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (i, label) in labels.iter().enumerate() {
            if creator::tab(ui, look, label, i == selected).clicked() {
                now = i;
            }
        }
    });
    now
}

/// One centred line of `text`, wrapped to the available width.
fn centred(ui: &mut Ui, text: &str, font: FontId, ink: Color32) {
    if text.is_empty() {
        return;
    }
    let width = ui.available_width();
    let galley = ui
        .painter()
        .layout(text.to_owned(), font, ink, width.max(40.0));
    let (rect, _) = ui.allocate_exact_size(vec2(width, galley.size().y), Sense::hover());
    let at = pos2(rect.center().x - galley.size().x / 2.0, rect.top());
    ui.painter().galley(at, galley, ink);
}

/// The About page. `mark` paints the app's mark in the square it is given
/// ([`measures::MARK`] points). Returns the index into [`Info::links`] of the link clicked.
pub fn page(
    ui: &mut Ui,
    look: &Look,
    info: &Info,
    mark: impl FnOnce(&mut Ui, Rect),
) -> Option<usize> {
    let p = &look.palette;
    let width = ui.available_width();
    ui.add_space(14.0);
    let (mark_row, _) = ui.allocate_exact_size(vec2(width, measures::MARK), Sense::hover());
    let square = Rect::from_center_size(mark_row.center(), vec2(measures::MARK, measures::MARK));
    mark(ui, square);
    ui.add_space(12.0);
    centred(ui, info.name, font(look, 26.0, true), color(p.text_primary));
    ui.add_space(2.0);
    centred(
        ui,
        info.version,
        font(look, 13.0, false),
        color(p.text_secondary),
    );
    ui.add_space(10.0);
    centred(
        ui,
        info.tagline,
        font(look, 13.0, false),
        color(p.text_secondary),
    );
    ui.add_space(4.0);
    centred(
        ui,
        info.suite,
        font(look, 12.0, false),
        color(p.text_tertiary),
    );
    ui.add_space(14.0);
    let picked = links(ui, look, info.links);
    if !info.footer.is_empty() {
        ui.add_space(16.0);
        centred(
            ui,
            info.footer,
            font(look, 11.5, false),
            color(p.text_tertiary),
        );
    }
    ui.add_space(8.0);
    picked
}

/// The links, centred in rows that wrap when the window is narrow.
fn links(ui: &mut Ui, look: &Look, links: &[Link]) -> Option<usize> {
    let width = ui.available_width();
    let galleys: Vec<_> = links
        .iter()
        .map(|l| {
            ui.painter().layout_no_wrap(
                l.label.to_owned(),
                font(look, 13.0, false),
                color(look.palette.text_link),
            )
        })
        .collect();
    let mut picked = None;
    let mut start = 0;
    while start < links.len() {
        let mut end = start;
        let mut used = 0.0;
        while let Some(g) = galleys.get(end) {
            let next = used + g.size().x + if end > start { measures::LINK_GAP } else { 0.0 };
            if end > start && next > width {
                break;
            }
            used = next;
            end += 1;
        }
        let (row, _) = ui.allocate_exact_size(vec2(width, 22.0), Sense::hover());
        let mut x = row.center().x - used / 2.0;
        for i in start..end {
            let (Some(g), Some(l)) = (galleys.get(i), links.get(i)) else {
                continue;
            };
            let rect = Rect::from_min_size(pos2(x, row.top()), vec2(g.size().x, row.height()));
            let resp = ui.interact(rect, ui.scope_id().with(("about.link", i)), Sense::click());
            link_widget_info(&resp, l.label);
            let ink = color(look.palette.text_link);
            ui.painter().galley(
                pos2(rect.left(), rect.center().y - g.size().y / 2.0),
                g.clone(),
                ink,
            );
            if resp.hovered() {
                let y = rect.center().y + g.size().y / 2.0;
                ui.painter().hline(rect.x_range(), y, Stroke::new(1.0, ink));
            }
            if resp
                .on_hover_text(l.url)
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                picked = Some(i);
            }
            x += g.size().x + measures::LINK_GAP;
        }
        start = end;
    }
    picked
}

fn link_widget_info(resp: &Response, label: &str) {
    resp.widget_info(|| WidgetInfo::labeled(Role::Link, true, label));
}
