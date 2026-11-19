//! NDL's volumetric CAD icons: the isometric 3D drawings the engineering apps (Torno, Vortia,
//! Forja) share for modelling and drafting operations, generated from
//! `packages/icons/volumetric` so every app draws the same artwork.
//!
//! An icon is a list of [`Part`]s on a 64 x 64 box. Parts are coloured by [`Role`], never by a
//! literal, so an app recolours finished artwork with its own [`Roles`]: the neutral faces, the
//! outline ink and the accent that marks the geometry an operation creates. With the `egui`
//! feature, [`paint`] draws one into an `egui::Painter`.
//!
//! ```
//! let trim = nodalia_theme::icons::volumetric::get("trim").expect("NDL has a trim icon");
//! assert_eq!(trim.group, "sketch");
//! assert!(nodalia_theme::icons::volumetric::get("no-such-icon").is_none());
//! ```

#[rustfmt::skip]
mod data;

#[cfg(feature = "egui")]
mod draw;
#[cfg(feature = "egui")]
pub use draw::{paint, Roles};

/// The side of the square box every icon is drawn in.
pub const BOX: f32 = 64.0;

/// The colour a part is drawn in, by what it means on the artwork.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Outlines and edges.
    Ink,
    /// Line art on the interface background rather than on a face.
    Line,
    /// The light, medium and dark neutral faces of a solid (top, left, right).
    Face1,
    Face2,
    Face3,
    /// The accent ramp: the geometry an operation creates (light fill, mid, dark line).
    Accent1,
    Accent2,
    Accent3,
    /// Construction references and guides.
    Guide,
    /// Glyphs on badges.
    Paper,
}

impl Role {
    /// Every role, in the order of [`Role::index`].
    pub const ALL: [Role; 10] = [
        Role::Ink,
        Role::Line,
        Role::Face1,
        Role::Face2,
        Role::Face3,
        Role::Accent1,
        Role::Accent2,
        Role::Accent3,
        Role::Guide,
        Role::Paper,
    ];

    /// The position of the role in [`Role::ALL`].
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// One drawn element: a path on the 64 box, filled and/or stroked.
#[derive(Debug, PartialEq)]
pub struct Part {
    /// The SVG path data (absolute M L C Q A Z).
    pub d: &'static str,
    pub fill: Option<Role>,
    pub stroke: Option<Role>,
    /// The stroke width on the 64 box.
    pub width: f32,
    /// The dash and gap of a dashed stroke, on the 64 box.
    pub dash: Option<(f32, f32)>,
}

/// One volumetric icon.
#[derive(Debug, PartialEq)]
pub struct Volumetric {
    /// Its kebab-case name, the same as the SVG file's.
    pub name: &'static str,
    /// The group it is listed under (`sketch`, `features`, `simulation`, ...).
    pub group: &'static str,
    /// The drawing, back to front.
    pub parts: &'static [Part],
}

/// Every volumetric icon, sorted by name.
pub fn all() -> &'static [Volumetric] {
    &data::ICONS
}

/// The volumetric icon called `name`.
pub fn get(name: &str) -> Option<&'static Volumetric> {
    let icons = all();
    icons
        .binary_search_by(|i| i.name.cmp(name))
        .ok()
        .and_then(|i| icons.get(i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_sorted_and_unique() {
        assert!(all().windows(2).all(|w| w[0].name < w[1].name));
        assert!(all().len() > 300);
    }

    #[test]
    fn every_icon_can_be_found_and_has_parts() {
        for icon in all() {
            assert_eq!(get(icon.name), Some(icon));
            assert!(!icon.parts.is_empty(), "{}", icon.name);
            for part in icon.parts {
                assert!(part.d.starts_with('M'), "{}: {}", icon.name, part.d);
                assert!(
                    part.fill.is_some() || part.stroke.is_some(),
                    "{}",
                    icon.name
                );
            }
        }
    }

    #[test]
    fn roles_are_indexed_in_order() {
        for (i, role) in Role::ALL.iter().enumerate() {
            assert_eq!(role.index(), i);
        }
    }
}
