//! A small Markdown renderer for the assistant's answers: headings, lists, quotes, code,
//! tables, bold, italic, inline code and links. Anything else is shown as the text it is.
//!
//! [`parse`] is plain data and has no egui in it; [`show`] draws the blocks.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId, Frame, Margin, RichText, Stroke, Ui};
use nodalia_theme::for_egui::color;
use nodalia_theme::office::Look;

use crate::lang::Lang;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Plain,
    Bold,
    Italic,
    Code,
    Link,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Heading(u8, Vec<Span>),
    Paragraph(Vec<Span>),
    /// A bullet at an indent level (two spaces each).
    Bullet(usize, Vec<Span>),
    /// A numbered item: its number as written, its indent level.
    Numbered(String, usize, Vec<Span>),
    Quote(Vec<Span>),
    Code(String),
    Table(Vec<Vec<String>>),
    Rule,
}

fn span(text: &str, style: Style) -> Span {
    Span {
        text: text.to_string(),
        style,
    }
}

/// The inline token at the start of `rest`, with what follows it. `before` is the plain text
/// already read, which decides whether an underscore is emphasis.
fn token<'a>(rest: &'a str, before: &str) -> Option<(Span, &'a str)> {
    if let Some(body) = rest.strip_prefix('`') {
        let end = body.find('`').filter(|e| *e > 0)?;
        return Some((span(&body[..end], Style::Code), &body[end + 1..]));
    }
    if let Some(body) = rest.strip_prefix("**") {
        if let Some(end) = body.find("**").filter(|e| *e > 0) {
            return Some((span(&body[..end], Style::Bold), &body[end + 2..]));
        }
    }
    for mark in ['*', '_'] {
        let Some(body) = rest.strip_prefix(mark) else {
            continue;
        };
        // `snake_case` and `2 * 3` are not emphasis.
        if mark == '_' && before.chars().last().is_some_and(char::is_alphanumeric) {
            continue;
        }
        let Some(end) = body.find(mark).filter(|e| *e > 0) else {
            continue;
        };
        let inner = &body[..end];
        if !inner.starts_with(char::is_whitespace) && !inner.ends_with(char::is_whitespace) {
            return Some((span(inner, Style::Italic), &body[end + 1..]));
        }
    }
    if let Some(body) = rest.strip_prefix('[') {
        let mid = body.find("](")?;
        let close = body[mid + 2..].find(')')?;
        return Some((
            span(&body[..mid], Style::Link),
            &body[mid + 2 + close + 1..],
        ));
    }
    None
}

/// The inline styles of one run of text.
pub fn spans(src: &str) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    let mut plain = String::new();
    let mut rest = src;
    while let Some(c) = rest.chars().next() {
        if let Some((found, after)) = token(rest, &plain) {
            if !plain.is_empty() {
                out.push(span(&plain, Style::Plain));
                plain.clear();
            }
            out.push(found);
            rest = after;
        } else {
            plain.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    if !plain.is_empty() {
        out.push(span(&plain, Style::Plain));
    }
    out
}

fn list_marker(line: &str) -> Option<(usize, &str)> {
    let indent = line.len() - line.trim_start().len();
    let body = line.trim_start();
    for m in ["- ", "* ", "+ "] {
        if let Some(r) = body.strip_prefix(m) {
            return Some((indent / 2, r));
        }
    }
    None
}

fn number_marker(line: &str) -> Option<(String, usize, &str)> {
    let indent = line.len() - line.trim_start().len();
    let body = line.trim_start();
    let digits = body.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > 3 {
        return None;
    }
    let after = &body[digits..];
    let rest = after
        .strip_prefix(". ")
        .or_else(|| after.strip_prefix(") "))?;
    Some((body[..digits].to_string(), indent / 2, rest))
}

fn is_rule(line: &str) -> bool {
    let t = line.trim();
    t.len() >= 3
        && (t.chars().all(|c| c == '-')
            || t.chars().all(|c| c == '*')
            || t.chars().all(|c| c == '_'))
}

fn cells(line: &str) -> Vec<String> {
    let t = line.trim().trim_start_matches('|').trim_end_matches('|');
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn is_table_separator(line: &str) -> bool {
    let t = line.trim();
    t.contains('-') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// Splits an answer into blocks.
pub fn parse(src: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut lines = src.lines().peekable();
    fn end(paragraph: &mut Vec<&str>, blocks: &mut Vec<Block>) {
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph(spans(&paragraph.join("\n"))));
            paragraph.clear();
        }
    }
    while let Some(line) = lines.next() {
        let t = line.trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            end(&mut paragraph, &mut blocks);
            let mut code = Vec::new();
            for l in lines.by_ref() {
                if l.trim().starts_with("```") || l.trim().starts_with("~~~") {
                    break;
                }
                code.push(l);
            }
            blocks.push(Block::Code(code.join("\n")));
        } else if t.is_empty() {
            end(&mut paragraph, &mut blocks);
        } else if is_rule(t) {
            end(&mut paragraph, &mut blocks);
            blocks.push(Block::Rule);
        } else if let Some(h) = t.strip_prefix('#').map(|_| t.trim_start_matches('#')) {
            let level = t.len() - h.len();
            if (1..=6).contains(&level) && h.starts_with(' ') {
                end(&mut paragraph, &mut blocks);
                blocks.push(Block::Heading(level as u8, spans(h.trim())));
            } else {
                paragraph.push(line);
            }
        } else if let Some(q) = t.strip_prefix('>') {
            end(&mut paragraph, &mut blocks);
            blocks.push(Block::Quote(spans(q.trim())));
        } else if let Some((level, rest)) = list_marker(line) {
            end(&mut paragraph, &mut blocks);
            blocks.push(Block::Bullet(level, spans(rest.trim())));
        } else if let Some((n, level, rest)) = number_marker(line) {
            end(&mut paragraph, &mut blocks);
            blocks.push(Block::Numbered(n, level, spans(rest.trim())));
        } else if t.starts_with('|') && t.len() > 1 {
            end(&mut paragraph, &mut blocks);
            let mut rows = vec![cells(t)];
            while let Some(next) = lines.peek() {
                if !next.trim().starts_with('|') {
                    break;
                }
                if !is_table_separator(next) {
                    rows.push(cells(next));
                }
                lines.next();
            }
            blocks.push(Block::Table(rows));
        } else {
            paragraph.push(line);
        }
    }
    end(&mut paragraph, &mut blocks);
    blocks
}

fn font(look: &Look, size: f32, style: Style) -> FontId {
    match style {
        Style::Code => FontId::new(size - 1.0, FontFamily::Monospace),
        Style::Bold => FontId::new(size, look.bold.clone()),
        _ => FontId::new(size, look.regular.clone()),
    }
}

fn job(look: &Look, spans: &[Span], size: f32, ink: Color32, wrap: f32) -> LayoutJob {
    let p = &look.palette;
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    for s in spans {
        let mut format = TextFormat {
            font_id: font(look, size, s.style),
            color: ink,
            line_height: Some(size * 1.5),
            ..TextFormat::default()
        };
        match s.style {
            Style::Italic => format.italics = true,
            Style::Code => {
                format.background = color(p.surface_sunken);
            }
            Style::Link => {
                format.color = color(p.text_link);
                format.underline = Stroke::new(1.0, color(p.text_link));
            }
            Style::Plain | Style::Bold => {}
        }
        job.append(&s.text, 0.0, format);
    }
    job
}

fn text(ui: &mut Ui, look: &Look, spans: &[Span], size: f32, ink: Color32) {
    let wrap = ui.available_width();
    ui.add(egui::Label::new(job(look, spans, size, ink, wrap)).wrap());
}

/// Draws `markdown` in the available width.
pub fn show(ui: &mut Ui, look: &Look, lang: Lang, id: &str, markdown: &str) {
    let p = &look.palette;
    let ink = color(p.text_primary);
    let size = 14.0;
    for (i, block) in parse(markdown).iter().enumerate() {
        match block {
            Block::Paragraph(s) => text(ui, look, s, size, ink),
            Block::Heading(level, s) => {
                let bold: Vec<Span> = s
                    .iter()
                    .map(|x| Span {
                        text: x.text.clone(),
                        style: if x.style == Style::Plain {
                            Style::Bold
                        } else {
                            x.style
                        },
                    })
                    .collect();
                ui.add_space(4.0);
                text(
                    ui,
                    look,
                    &bold,
                    if *level <= 2 { 16.0 } else { size + 1.0 },
                    ink,
                );
            }
            Block::Bullet(level, s) => {
                ui.horizontal_top(|ui| {
                    ui.add_space(6.0 + *level as f32 * 14.0);
                    ui.label(RichText::new("•").color(color(p.text_secondary)));
                    text(ui, look, s, size, ink);
                });
            }
            Block::Numbered(n, level, s) => {
                ui.horizontal_top(|ui| {
                    ui.add_space(4.0 + *level as f32 * 14.0);
                    ui.label(RichText::new(format!("{n}.")).color(color(p.text_secondary)));
                    text(ui, look, s, size, ink);
                });
            }
            Block::Quote(s) => {
                Frame::new()
                    .inner_margin(Margin {
                        left: 10,
                        ..Margin::ZERO
                    })
                    .stroke(Stroke::NONE)
                    .show(ui, |ui| text(ui, look, s, size, color(p.text_secondary)));
            }
            Block::Rule => {
                ui.separator();
            }
            Block::Code(code) => {
                Frame::new()
                    .fill(color(p.surface_sunken))
                    .corner_radius(8)
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.add(egui::Label::new(
                            RichText::new(code)
                                .font(FontId::new(12.5, FontFamily::Monospace))
                                .color(ink),
                        ));
                        if ui.small_button(lang.t("Copy")).clicked() {
                            ui.ctx().copy_text(code.clone());
                        }
                    });
            }
            Block::Table(rows) => {
                egui::Grid::new((id, "table", i))
                    .striped(true)
                    .spacing(egui::vec2(14.0, 6.0))
                    .show(ui, |ui| {
                        for (r, row) in rows.iter().enumerate() {
                            for cell in row {
                                let s = spans(cell);
                                let s: Vec<Span> = if r == 0 {
                                    s.into_iter()
                                        .map(|x| Span {
                                            style: if x.style == Style::Plain {
                                                Style::Bold
                                            } else {
                                                x.style
                                            },
                                            ..x
                                        })
                                        .collect()
                                } else {
                                    s
                                };
                                ui.add(egui::Label::new(job(look, &s, 13.0, ink, 240.0)).wrap());
                            }
                            ui.end_row();
                        }
                    });
            }
        }
        ui.add_space(6.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(s: &[Span]) -> String {
        s.iter().map(|x| x.text.as_str()).collect()
    }

    #[test]
    fn inline_styles_are_found() {
        let s = spans("a **b** *c* `d` [e](http://x) f_g_h");
        let styles: Vec<_> = s.iter().map(|x| x.style).collect();
        assert_eq!(
            styles,
            vec![
                Style::Plain,
                Style::Bold,
                Style::Plain,
                Style::Italic,
                Style::Plain,
                Style::Code,
                Style::Plain,
                Style::Link,
                Style::Plain
            ]
        );
        assert_eq!(plain(&s), "a b c d e f_g_h");
    }

    #[test]
    fn stray_marks_stay_text() {
        for src in [
            "2 * 3 * 4",
            "a ** b",
            "`open",
            "[x](",
            "snake_case_name",
            "*",
            "**",
            "_",
        ] {
            assert_eq!(plain(&spans(src)), src, "{src}");
        }
        assert_eq!(plain(&spans("ñandú **ñ** é")), "ñandú ñ é");
    }

    #[test]
    fn blocks_are_split_by_their_markers() {
        let src = "# Title\n\nSome *text*\nover two lines.\n\n- one\n  - nested\n1. first\n2) second\n\n> quoted\n\n```rust\nlet x = 1;\n```\n\n---\n| a | b |\n|---|---|\n| 1 | 2 |\n";
        let b = parse(src);
        assert!(matches!(&b[0], Block::Heading(1, s) if plain(s) == "Title"));
        assert!(matches!(&b[1], Block::Paragraph(s) if plain(s) == "Some text\nover two lines."));
        assert!(matches!(&b[2], Block::Bullet(0, s) if plain(s) == "one"));
        assert!(matches!(&b[3], Block::Bullet(1, s) if plain(s) == "nested"));
        assert!(matches!(&b[4], Block::Numbered(n, 0, _) if n == "1"));
        assert!(matches!(&b[5], Block::Numbered(n, 0, _) if n == "2"));
        assert!(matches!(&b[6], Block::Quote(_)));
        assert!(matches!(&b[7], Block::Code(c) if c == "let x = 1;"));
        assert_eq!(b[8], Block::Rule);
        assert!(matches!(&b[9], Block::Table(r) if r.len() == 2 && r[1] == vec!["1", "2"]));
    }

    #[test]
    fn an_unclosed_fence_takes_the_rest_and_hashes_without_a_space_are_text() {
        let b = parse("```\ncode\nmore");
        assert_eq!(b, vec![Block::Code("code\nmore".into())]);
        let b = parse("#hashtag");
        assert!(matches!(&b[0], Block::Paragraph(s) if plain(s) == "#hashtag"));
        assert!(parse("").is_empty());
    }

    #[test]
    fn hostile_text_never_panics() {
        for src in [
            "|",
            "||",
            "| |",
            "####### x",
            "1.",
            "999. x",
            "[](",
            "``",
            "**``**",
            "\u{0}",
        ] {
            let _ = parse(src);
        }
    }
}
