//! The providers' own marks, drawn from alpha masks in `assets/marks/`.
//!
//! Each mask is the provider's mark rendered once from its single-colour SVG (the SVGs sit beside
//! them) at 96 × 96 pixels, one byte of coverage per pixel. [`draw`] uploads it as a texture and
//! tints it with the colour the interface asks for, so a mark follows the theme and is always a
//! single flat colour, unaltered otherwise. The marks stay the property of their owners; see
//! `assets/marks/README.md` for their origin and licences.

use egui::{pos2, Color32, ColorImage, Context, Id, Painter, Rect, TextureHandle, TextureOptions};

use crate::provider::Provider;

/// The side of every mask, in pixels.
const SIDE: usize = 96;

/// The coverage mask of `provider`'s mark, or `None` for a provider that has no mark of its own.
fn mask(provider: Provider) -> Option<&'static [u8]> {
    let bytes: &'static [u8] = match provider {
        Provider::Claude => include_bytes!("../../assets/marks/claude.a8"),
        Provider::OpenAi => include_bytes!("../../assets/marks/openai.a8"),
        Provider::Gemini => include_bytes!("../../assets/marks/gemini.a8"),
        Provider::Mistral => include_bytes!("../../assets/marks/mistral.a8"),
        Provider::OpenRouter => include_bytes!("../../assets/marks/openrouter.a8"),
        Provider::Ollama => include_bytes!("../../assets/marks/ollama.a8"),
        Provider::Custom => return None,
    };
    (bytes.len() == SIDE * SIDE).then_some(bytes)
}

/// Whether `provider` has a mark of its own.
pub fn has_mark(provider: Provider) -> bool {
    mask(provider).is_some()
}

fn texture(ctx: &Context, provider: Provider) -> Option<TextureHandle> {
    let id = Id::unique(("nodalia-assistant-mark", provider.id()));
    if let Some(found) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return Some(found);
    }
    let coverage = mask(provider)?;
    let mut rgba = Vec::with_capacity(coverage.len() * 4);
    for a in coverage {
        rgba.extend_from_slice(&[255, 255, 255, *a]);
    }
    let image = ColorImage::from_rgba_unmultiplied([SIDE, SIDE], &rgba);
    let handle = ctx.load_texture(id.short_debug_format(), image, TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, handle.clone()));
    Some(handle)
}

/// Draws `provider`'s mark inside `rect` in `ink`. Returns false when it has none.
pub fn draw(painter: &Painter, rect: Rect, provider: Provider, ink: Color32) -> bool {
    let Some(handle) = texture(painter.ctx(), provider) else {
        return false;
    };
    let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    painter.image(handle.id(), rect, whole, ink);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_provider_but_the_custom_server_has_a_whole_mask() {
        for p in Provider::ALL {
            if p == Provider::Custom {
                assert!(!has_mark(p), "a company gateway has no mark");
                continue;
            }
            let m = mask(p).unwrap_or_else(|| panic!("{} has no usable mask", p.id()));
            let inked = m.iter().filter(|a| **a > 128).count();
            let share = inked * 100 / m.len();
            assert!((10..=70).contains(&share), "{}: {share}% inked", p.id());
        }
    }

    #[test]
    fn the_marks_differ() {
        let mut seen: Vec<&[u8]> = Vec::new();
        for p in Provider::ALL {
            if let Some(m) = mask(p) {
                assert!(!seen.contains(&m), "{} repeats another mark", p.id());
                seen.push(m);
            }
        }
    }

    #[test]
    fn a_mark_draws_in_a_frame_and_the_custom_server_does_not() {
        let ctx = Context::default();
        let mut drawn = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(24.0, 24.0));
            for p in Provider::ALL {
                drawn.push((p, draw(ui.painter(), rect, p, Color32::WHITE)));
            }
        })
        .drop_without_applying_deltas();
        for (p, ok) in drawn {
            assert_eq!(ok, p != Provider::Custom, "{}", p.id());
        }
    }
}
