//! Draws volumetric icons with egui: each part's path flattened once (cached per icon), filled
//! even-odd and stroked with round caps and joins, coloured through [`Roles`].

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, PoisonError},
};

use egui::{epaint::Mesh, pos2, Color32, Painter, Pos2, Rect, Shape, Stroke};

use super::{Part, Role, Volumetric, BOX};
use crate::icons::paint::{convex, parse_path, regions, triangulate, Outline, Pt};

/// The colours of the roles for one surface. Start from [`Roles::light`] or [`Roles::dark`]
/// and change what the app's palette owns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Roles(pub [Color32; 10]);

impl Roles {
    /// NDL's display colours for a light interface.
    pub fn light() -> Self {
        let c = Color32::from_rgb;
        Roles([
            c(0x26, 0x30, 0x3d),
            c(0x28, 0x32, 0x3f),
            c(0xec, 0xee, 0xf2),
            c(0xc3, 0xc7, 0xd1),
            c(0x94, 0x99, 0xa8),
            c(0xf3, 0xe4, 0xac),
            c(0xd1, 0xb4, 0x53),
            c(0x8f, 0x75, 0x18),
            c(0x7a, 0x81, 0x93),
            c(0xff, 0xff, 0xff),
        ])
    }

    /// NDL's display colours for a dark interface: outlines stay dark, only line art on the
    /// background turns light.
    pub fn dark() -> Self {
        let c = Color32::from_rgb;
        Roles([
            c(0x12, 0x14, 0x1a),
            c(0xdf, 0xe2, 0xea),
            c(0xb0, 0xb5, 0xc3),
            c(0x86, 0x8c, 0x9f),
            c(0x61, 0x67, 0x7a),
            c(0xe8, 0xcf, 0x7c),
            c(0xc8, 0xa6, 0x3d),
            c(0x9b, 0x7e, 0x1d),
            c(0x95, 0x9c, 0xad),
            c(0x2b, 0x2e, 0x38),
        ])
    }

    /// The same colours with the accent ramp replaced (light fill, mid, dark line).
    pub fn with_accent(mut self, light: Color32, mid: Color32, dark: Color32) -> Self {
        self.0[Role::Accent1.index()] = light;
        self.0[Role::Accent2.index()] = mid;
        self.0[Role::Accent3.index()] = dark;
        self
    }

    /// The same colours with the line art on the background (`Role::Line`) in `color`.
    pub fn with_line(mut self, color: Color32) -> Self {
        self.0[Role::Line.index()] = color;
        self
    }

    /// The colour of `role`.
    pub fn get(&self, role: Role) -> Color32 {
        self.0
            .get(role.index())
            .copied()
            .unwrap_or(Color32::MAGENTA)
    }
}

/// One filled region: a convex polygon, or a triangulated one.
struct Region {
    poly: Vec<Pt>,
    triangles: Option<Vec<[u32; 3]>>,
    outer: Vec<Pt>,
}

/// A part, flattened.
struct Shapes {
    regions: Vec<Region>,
    outlines: Vec<Outline>,
}

type Cache = Mutex<HashMap<&'static str, Option<Arc<Vec<Shapes>>>>>;

/// The flattened parts of `icon`, computed once. `None` when a path is outside what the reader
/// understands.
fn shapes(icon: &'static Volumetric) -> Option<Arc<Vec<Shapes>>> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut map = cache.lock().unwrap_or_else(PoisonError::into_inner);
    map.entry(icon.name)
        .or_insert_with(|| flatten(icon).map(Arc::new))
        .clone()
}

fn flatten(icon: &Volumetric) -> Option<Vec<Shapes>> {
    icon.parts
        .iter()
        .map(|part| {
            let outlines = parse_path(part.d)?;
            let regions = if part.fill.is_some() {
                regions(&outlines)
                    .into_iter()
                    .map(|(poly, holes, outer)| {
                        let triangles = (holes || !convex(&poly)).then(|| triangulate(&poly));
                        Region {
                            poly,
                            triangles,
                            outer: outer.to_vec(),
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };
            Some(Shapes { regions, outlines })
        })
        .collect()
}

/// Paint `icon` into the square `rect`, coloured by `roles`. False (nothing painted) when a
/// path is outside the subset the reader understands.
pub fn paint(p: &Painter, rect: Rect, icon: &'static Volumetric, roles: &Roles) -> bool {
    let Some(shapes) = shapes(icon) else {
        return false;
    };
    let s = rect.width() / BOX;
    let at = |(x, y): Pt| pos2(rect.min.x + x * s, rect.min.y + y * s);
    for (part, shape) in icon.parts.iter().zip(shapes.iter()) {
        if let Some(role) = part.fill {
            fill(p, part, shape, &at, s, roles.get(role));
        }
        if let Some(role) = part.stroke {
            stroke(p, part, shape, &at, s, roles.get(role));
        }
    }
    true
}

fn fill(
    p: &Painter,
    part: &Part,
    shape: &Shapes,
    at: &impl Fn(Pt) -> Pos2,
    s: f32,
    color: Color32,
) {
    for region in &shape.regions {
        let screen: Vec<Pos2> = region.poly.iter().copied().map(at).collect();
        let Some(triangles) = &region.triangles else {
            p.add(fan(&dedupe(&screen), color));
            continue;
        };
        let mut mesh = Mesh::default();
        for &v in &screen {
            mesh.colored_vertex(v, color);
        }
        for &[a, b, c] in triangles {
            mesh.add_triangle(a, b, c);
        }
        p.add(Shape::mesh(mesh));
        if part.stroke.is_none() {
            // Without a stroke over it, a triangulated fill's edge needs a hairline to be smooth.
            let edge: Vec<Pos2> = region.outer.iter().copied().map(at).collect();
            let width = 0.5 * s.max(1.0);
            polyline(p, &dedupe(&edge), true, Stroke::new(width, color));
        }
    }
}

/// The points without repeats: a zero-length edge has no direction, and egui's anti-aliasing
/// stretches it into a spike across the screen.
fn dedupe(pts: &[Pos2]) -> Vec<Pos2> {
    let mut out: Vec<Pos2> = Vec::with_capacity(pts.len());
    for &q in pts {
        if out.last().is_none_or(|l| l.distance(q) > 0.02) {
            out.push(q);
        }
    }
    while out.len() > 1
        && out
            .first()
            .zip(out.last())
            .is_some_and(|(a, b)| a.distance(*b) <= 0.02)
    {
        out.pop();
    }
    out
}

/// A convex polygon as a triangle fan. Not `Shape::convex_polygon`: its anti-aliasing extrudes
/// every corner along the bisector, so the thin acute slivers these icons are made of grow spikes.
fn fan(pts: &[Pos2], color: Color32) -> Shape {
    let mut mesh = Mesh::default();
    for &v in pts {
        mesh.colored_vertex(v, color);
    }
    for i in 1..pts.len().saturating_sub(1) {
        if let (Ok(b), Ok(c)) = (u32::try_from(i), u32::try_from(i + 1)) {
            mesh.add_triangle(0, b, c);
        }
    }
    Shape::mesh(mesh)
}

/// A stroked polyline, cut at sharp turns and joined with discs.
fn polyline(p: &Painter, pts: &[Pos2], closed: bool, stroke: Stroke) {
    let mut line = pts.to_vec();
    if closed {
        line.extend(line.first().copied());
    }
    for (k, piece) in split_at_sharp_turns(&line).into_iter().enumerate() {
        if let Some(&joint) = piece.first().filter(|_| k > 0 || closed) {
            p.circle_filled(joint, stroke.width / 2.0, stroke.color);
        }
        p.add(Shape::line(piece, stroke));
    }
}

/// The polyline cut into pieces at every sharp turn: egui extrudes a stroke's corner along the
/// bisector without a limit, so a near-reversal (the cusp where a cylinder's silhouette meets its
/// rim) shoots a spike across the screen. The cut corners are covered by the round joins.
fn split_at_sharp_turns(pts: &[Pos2]) -> Vec<Vec<Pos2>> {
    let mut pieces = Vec::new();
    let mut piece: Vec<Pos2> = Vec::new();
    for (i, &q) in pts.iter().enumerate() {
        piece.push(q);
        let sharp = match (i.checked_sub(1).and_then(|k| pts.get(k)), pts.get(i + 1)) {
            (Some(&a), Some(&c)) => {
                let (u, v) = (q - a, c - q);
                u.dot(v) < -0.25 * u.length() * v.length()
            }
            _ => false,
        };
        if sharp && piece.len() >= 2 {
            pieces.push(std::mem::replace(&mut piece, vec![q]));
        }
    }
    if piece.len() >= 2 {
        pieces.push(piece);
    }
    pieces
}

fn stroke(
    p: &Painter,
    part: &Part,
    shape: &Shapes,
    at: &impl Fn(Pt) -> Pos2,
    s: f32,
    color: Color32,
) {
    let w = (part.width * s).max(0.5);
    let stroke = Stroke::new(w, color);
    for o in &shape.outlines {
        let pts: Vec<Pos2> = o.pts.iter().copied().map(at).collect();
        let line_pts = dedupe(&pts);
        if let Some((dash, gap)) = part.dash {
            let mut line = line_pts.clone();
            if o.closed {
                line.extend(line_pts.first().copied());
            }
            p.extend(Shape::dashed_line(&line, stroke, dash * s, gap * s));
            continue;
        }
        polyline(p, &line_pts, o.closed, stroke);
        // Round caps and joins: a disc of the stroke's width on every corner.
        for &i in &o.corners {
            if let Some(&c) = pts.get(i) {
                p.circle_filled(c, w / 2.0, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_flattens() {
        for icon in super::super::all() {
            assert!(shapes(icon).is_some(), "{}", icon.name);
        }
    }

    #[test]
    fn no_flattened_point_leaves_the_icon_box() {
        for icon in super::super::all() {
            for shape in shapes(icon).expect("flattens").iter() {
                for outline in &shape.outlines {
                    for &(x, y) in &outline.pts {
                        assert!(
                            (-8.0..=BOX + 8.0).contains(&x) && (-8.0..=BOX + 8.0).contains(&y),
                            "{} has a point at {x},{y}",
                            icon.name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn tessellated_icons_stay_inside_their_square() {
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(pos2(100.0, 100.0), egui::vec2(30.0, 30.0));
        let mut bad = Vec::new();
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        for icon in super::super::all() {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let painter = ui.painter().clone();
                assert!(paint(&painter, rect, icon, &Roles::dark()));
            });
            out.textures_delta.clear();
            let prims = ctx.tessellate(out.shapes, 1.0);
            let wide = rect.expand(6.0);
            for prim in prims {
                if let egui::epaint::Primitive::Mesh(mesh) = prim.primitive {
                    if mesh.vertices.iter().any(|v| !wide.contains(v.pos)) {
                        bad.push(icon.name);
                        break;
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{bad:?}");
    }

    #[test]
    fn the_roles_match_the_generator() {
        let light = Roles::light();
        assert_eq!(light.get(Role::Face1), Color32::from_rgb(0xec, 0xee, 0xf2));
        assert_eq!(
            Roles::dark().get(Role::Line),
            Color32::from_rgb(0xdf, 0xe2, 0xea)
        );
        let accented = light.with_accent(Color32::RED, Color32::GREEN, Color32::BLUE);
        assert_eq!(accented.get(Role::Accent2), Color32::GREEN);
    }

    #[test]
    fn painting_adds_shapes() {
        let ctx = egui::Context::default();
        let layer = egui::LayerId::background();
        let painter = Painter::new(
            ctx,
            layer,
            Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(64.0, 64.0)),
        );
        let trim = super::super::get("trim").expect("trim");
        assert!(paint(&painter, painter.clip_rect(), trim, &Roles::light()));
    }
}
