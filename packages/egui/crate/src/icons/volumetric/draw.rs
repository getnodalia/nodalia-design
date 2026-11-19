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
            p.add(Shape::convex_polygon(screen, color, Stroke::NONE));
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
            p.add(Shape::closed_line(
                edge,
                Stroke::new(0.5 * s.max(1.0), color),
            ));
        }
    }
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
        if let Some((dash, gap)) = part.dash {
            let mut line = pts.clone();
            if o.closed {
                line.extend(pts.first().copied());
            }
            p.extend(Shape::dashed_line(&line, stroke, dash * s, gap * s));
            continue;
        }
        if o.closed {
            p.add(Shape::closed_line(pts.clone(), stroke));
        } else {
            p.add(Shape::line(pts.clone(), stroke));
        }
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
