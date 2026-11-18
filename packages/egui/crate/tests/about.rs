#![cfg(feature = "creator")]

use egui::vec2;
use nodalia_theme::about::{self, measures, Info, Link};
use nodalia_theme::creator::{layout, Look};
use nodalia_theme::{Accent, Mode};

fn run(mut draw: impl FnMut(&mut egui::Ui)) {
    let ctx = egui::Context::default();
    for _ in 0..2 {
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| draw(ui));
        out.textures_delta.clear();
    }
}

fn look(mode: Mode) -> Look {
    Look::new(mode.palette_with(Accent::Pulso))
}

const LINKS: [Link<'static>; 3] = [
    Link {
        label: "Website",
        url: "https://getnodalia.com/pulso",
    },
    Link {
        label: "Source code",
        url: "https://github.com/getnodalia/pulso",
    },
    Link {
        label: "Report an issue",
        url: "https://github.com/getnodalia/pulso/issues",
    },
];

fn info() -> Info<'static> {
    Info {
        name: "Pulso",
        version: "Version 0.1.1",
        tagline: "Motion graphics and visual effects, native on macOS, Windows and Linux.",
        suite: "Part of Nodalia · Creator",
        links: &LINKS,
        footer: "MIT OR Apache-2.0",
    }
}

#[test]
fn the_window_draws_in_both_modes_and_nothing_is_clicked_untouched() {
    for mode in [Mode::Light, Mode::Dark] {
        let look = look(mode);
        run(|ui| {
            ui.set_width(measures::WIDTH);
            assert!(!about::heading(
                ui,
                &look,
                "About Pulso",
                egui::Id::NULL.with("x")
            ));
            assert_eq!(about::tabs(ui, &look, &["About", "Contributors"], 1), 1);
            let mut mark = None;
            assert_eq!(
                about::page(ui, &look, &info(), |_, rect| mark = Some(rect)),
                None
            );
            let rect = mark.expect("the page paints the mark");
            assert_eq!(rect.size(), vec2(measures::MARK, measures::MARK));
        });
    }
}

#[test]
fn the_heading_is_as_tall_as_its_close_button() {
    let look = look(Mode::Light);
    let mut height = 0.0;
    run(|ui| {
        ui.set_width(300.0);
        let before = ui.cursor().top();
        about::heading(ui, &look, "About", egui::Id::NULL.with("h"));
        height = ui.cursor().top() - before;
    });
    assert!(height >= layout::CLOSE);
}

#[test]
fn links_wrap_inside_a_narrow_window_instead_of_overflowing() {
    let look = look(Mode::Light);
    let mut wide = 0.0;
    let mut narrow = 0.0;
    for (width, out) in [(measures::WIDTH, &mut wide), (150.0, &mut narrow)] {
        run(|ui| {
            ui.set_width(width);
            let before = ui.cursor().top();
            about::page(ui, &look, &info(), |_, _| {});
            *out = ui.cursor().top() - before;
        });
    }
    assert!(narrow > wide, "narrow {narrow} wide {wide}");
}

#[test]
fn an_empty_footer_and_no_links_still_draw() {
    let look = look(Mode::Dark);
    run(|ui| {
        ui.set_width(measures::WIDTH);
        let bare = Info {
            links: &[],
            footer: "",
            suite: "",
            ..info()
        };
        assert_eq!(about::page(ui, &look, &bare, |_, _| {}), None);
    });
}
