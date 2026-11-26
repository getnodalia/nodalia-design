#![cfg(feature = "enterprise")]

use egui::{pos2, vec2, Rect};
use nodalia_theme::creator::Look;
use nodalia_theme::enterprise::{self as ent, integrations, Account, Count, Panes, Row, Tone};
use nodalia_theme::{Accent, Mode};

fn run(mut draw: impl FnMut(&mut egui::Ui)) {
    let ctx = egui::Context::default();
    for _ in 0..2 {
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| draw(ui));
        out.textures_delta.clear();
    }
}

fn look(mode: Mode, accent: Accent) -> Look {
    Look::new(mode.palette_with(accent))
}

fn window(width: f32, height: f32) -> Rect {
    Rect::from_min_size(pos2(0.0, 0.0), vec2(width, height))
}

#[test]
fn the_panes_tile_the_window_without_overlap() {
    let gap = ent::layout::GAP;
    let panes = Panes::new(window(1280.0, 800.0), Some(ent::layout::DETAIL));
    let detail = panes.detail.expect("a wide window has room for the detail");
    assert!(!panes.squeezed);
    assert_eq!(panes.bar.height(), ent::layout::APP_BAR);
    assert_eq!(panes.side.width(), ent::layout::SIDEBAR);
    assert_eq!(detail.width(), ent::layout::DETAIL);
    assert_eq!(panes.side.right() + gap, panes.center.left());
    assert_eq!(panes.center.right() + gap, detail.left());
    assert_eq!(detail.right(), 1280.0 - gap);
    assert_eq!(panes.side.bottom(), 800.0 - gap);
    assert!(panes.center.width() >= ent::layout::MIN_PAGE);
}

#[test]
fn without_a_detail_the_page_takes_the_rest() {
    let panes = Panes::new(window(1000.0, 700.0), None);
    assert!(panes.detail.is_none() && !panes.squeezed);
    assert_eq!(panes.center.right(), 1000.0 - ent::layout::GAP);
}

#[test]
fn a_narrow_window_gives_the_detail_up_and_says_so() {
    let panes = Panes::new(window(700.0, 600.0), Some(ent::layout::DETAIL));
    assert!(panes.detail.is_none());
    assert!(panes.squeezed, "the app draws the detail over the page");
    assert!(panes.center.width() > 0.0);
}

#[test]
fn a_tiny_window_never_produces_a_reversed_rectangle() {
    for width in [0.0, 10.0, 200.0, 280.0] {
        for height in [0.0, 30.0, 400.0] {
            let panes = Panes::new(window(width, height), Some(ent::layout::DETAIL));
            for rect in [panes.bar, panes.side, panes.center] {
                assert!(
                    rect.width() >= 0.0 && rect.height() >= 0.0,
                    "{width}x{height}"
                );
            }
        }
    }
}

#[test]
fn every_part_draws_in_both_modes_for_both_apps() {
    for mode in [Mode::Light, Mode::Dark] {
        for accent in [Accent::Agora, Accent::Hito] {
            let look = look(mode, accent);
            let mut sizes = Vec::new();
            run(|ui| {
                sizes.clear();
                ui.set_width(240.0);
                let me = Account {
                    seed: 3,
                    name: "Ana García",
                    detail: "@ana · Online",
                    online: Some(true),
                };
                let panes = Panes::new(window(1100.0, 700.0), Some(ent::layout::DETAIL));
                ent::island(ui, &look, panes.side, "side", |ui| {
                    ent::sidebar(ui, &look, &me, |ui| {
                        ent::workspace_switcher(ui, &look, "Nodalia Studio", "Workspaces", |_| {});
                        ent::section_heading(ui, &look, "Channels", Some("Create a channel"));
                        sizes.push(
                            Row::new("general")
                                .icon("hash")
                                .selected(true)
                                .show(ui, &look)
                                .rect
                                .size(),
                        );
                        sizes.push(
                            Row::new("Ana")
                                .avatar(1, Some(false))
                                .bold(true)
                                .count(Count::Strong(3))
                                .show(ui, &look)
                                .rect
                                .size(),
                        );
                        sizes.push(
                            Row::new("Overview")
                                .icon("home")
                                .count(Count::Quiet(120))
                                .muted(true)
                                .show(ui, &look)
                                .rect
                                .size(),
                        );
                    });
                });
                let mut q = String::new();
                ent::search_field(ui, &look, &mut q, "Search", 220.0);
                ent::status_pill(ui, &look, Tone::Success, "Connected");
                ent::status_line(ui, &look, "Registration is closed.", true);
                ent::progress_bar(ui, &look, 0.4, false, 0.0, Some("40 %"));
                ent::progress_bar(ui, &look, f32::NAN, true, 80.0, None);
                ent::chip(ui, "web", egui::Color32::BLACK, egui::Color32::WHITE);
                ent::badge(ui, &look, 0, false);
                ent::empty_state(ui, &look, "inbox", "Nothing here yet.");
                assert!(!ent::panel_header(ui, &look, "Details", true));
                assert!(!ent::icon_button(ui, &look, "plus", "Add", false).clicked());
                assert!(!ent::danger_button(ui, &look, "Delete", false).clicked());
                ent::field_label(ui, &look, "Server");
                let mut text = String::new();
                ent::field(ui, &look, &mut text, "chat.example.org", false);
                ent::field(ui, &look, &mut text, "", true);
                ent::toasts(ui, &look, ["Saved", "Copied"].into_iter());
                assert_eq!(
                    ent::picker(
                        ui,
                        &look,
                        "lang",
                        "English",
                        &["English", "Español"],
                        Some(0),
                        180.0
                    ),
                    None
                );
                integrations::service_card(ui, &look, "link", "Hito", "last notice today", |ui| {
                    ui.label("actions");
                });
                integrations::numbered(ui, &look, 1, "Open the channel's settings.");
                assert!(!integrations::copy_row(
                    ui,
                    &look,
                    "Address",
                    "https://x",
                    "Copy"
                ));
                integrations::note(ui, &look, "Shown once.");
                integrations::step_label(ui, &look, "Step 1 of 3");
                integrations::heading(ui, &look, "Connect");
                integrations::link_pill(ui, &look, integrations::Link::Off, "Not connected");
                integrations::link_pill(ui, &look, integrations::Link::Failing, "Failing");
            });
            for size in &sizes {
                assert_eq!(
                    size.y,
                    ent::layout::ROW,
                    "every sidebar row has the same height"
                );
            }
        }
    }
}

#[test]
fn the_sign_in_card_has_the_suite_width() {
    let look = look(Mode::Light, Accent::Agora);
    let mut width = 0.0;
    run(|ui| {
        ent::sign_in_page(ui, window(900.0, 700.0), 600.0, |ui| {
            width = ent::sign_in_card(ui, &look, |ui| {
                ui.label("form");
                ui.min_rect().width()
            });
        });
    });
    assert!(
        (width - (ent::layout::SIGN_IN - 48.0)).abs() < 0.5,
        "{width}"
    );
}

#[test]
fn tones_keep_text_readable_on_their_background() {
    for mode in [Mode::Light, Mode::Dark] {
        for accent in [Accent::Agora, Accent::Hito] {
            let look = look(mode, accent);
            for tone in [Tone::Success, Tone::Info, Tone::Warning, Tone::Danger] {
                let (fg, bg) = tone.colors(&look);
                let ratio = nodalia_theme::contrast(
                    nodalia_theme::Rgba::new(fg.r(), fg.g(), fg.b(), 255),
                    nodalia_theme::Rgba::new(bg.r(), bg.g(), bg.b(), 255),
                );
                assert!(ratio >= 3.0, "{tone:?} in {mode:?}: {ratio:.2}");
            }
        }
    }
}

#[test]
fn the_window_frame_fits_the_platform() {
    use ent::chrome;
    let viewport = chrome::frame(egui::ViewportBuilder::default(), false);
    if cfg!(target_os = "macos") {
        assert_eq!(viewport.fullsize_content_view, Some(true));
        assert_eq!(viewport.titlebar_shown, Some(false));
        assert_ne!(viewport.titlebar_buttons_shown, Some(false));
        assert!(chrome::leading_inset() > 0.0 && chrome::drags_window(false));
    } else {
        assert_eq!(viewport.fullsize_content_view, None);
        assert_eq!(viewport.decorations, Some(true));
        assert_eq!(chrome::leading_inset(), 0.0);
        assert!(!chrome::drags_window(false));
        assert_eq!(
            chrome::frame(egui::ViewportBuilder::default(), true).decorations,
            Some(false)
        );
    }
}

#[test]
fn controls_map_to_viewport_commands() {
    use egui::ViewportCommand;
    use nodalia_theme::window_controls::Action;
    let command = ent::chrome::command;
    assert_eq!(command(Action::Minimize), ViewportCommand::Minimized(true));
    assert_eq!(command(Action::Maximize), ViewportCommand::Maximized(true));
    assert_eq!(command(Action::Restore), ViewportCommand::Maximized(false));
    assert_eq!(command(Action::Close), ViewportCommand::Close);
}

#[test]
fn resize_bands_stay_on_the_edges() {
    let full = window(800.0, 600.0);
    let inner = full.shrink(5.0);
    for (_, _, band) in ent::chrome::bands(full) {
        assert!(band.is_positive());
        assert!(full.contains_rect(band));
        assert!(!band.intersect(inner).is_positive());
    }
}

#[test]
fn a_connection_that_is_off_has_no_tone() {
    use integrations::Link;
    assert_eq!(Link::Off.tone(), None);
    assert_eq!(Link::Connected.tone(), Some(Tone::Success));
    assert_eq!(Link::Failing.tone(), Some(Tone::Danger));
}
