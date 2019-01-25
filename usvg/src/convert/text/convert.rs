// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::cmp;

// external
mod fk {
    pub use font_kit::source::SystemSource;
    pub use font_kit::properties::*;
    pub use font_kit::family_name::FamilyName;
    pub use font_kit::font::Font;
}

// self
use tree;
use super::super::prelude::*;


#[derive(Clone, Copy)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

#[derive(Clone)]
pub struct Font {
    pub font: fk::Font,
    pub size: f64,
    pub units_per_em: u32,
    pub ascent: f64,
    pub underline_position: f64,
    pub underline_thickness: f64,
    pub letter_spacing: f64,
    pub word_spacing: f64,
}

#[derive(Clone, Copy)]
pub struct CharacterPosition {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub dx: Option<f64>,
    pub dy: Option<f64>,
}

pub type PositionsList = Vec<CharacterPosition>;
pub type RotateList = Vec<f64>;

pub fn resolve_font(
    attrs: &svgdom::Attributes,
) -> Option<Font> {
    let style = attrs.get_str_or(AId::FontStyle, "normal");
    let style = match style {
        "normal"  => fk::Style::Normal,
        "italic"  => fk::Style::Italic,
        "oblique" => fk::Style::Oblique,
        _         => fk::Style::Normal,
    };

    let weight = attrs.get_str_or(AId::FontWeight, "normal");
    let weight = match weight {
        "normal" => fk::Weight::NORMAL,
        "bold"   => fk::Weight::BOLD,
        "100"    => fk::Weight::THIN,
        "200"    => fk::Weight::EXTRA_LIGHT,
        "300"    => fk::Weight::LIGHT,
        "400"    => fk::Weight::NORMAL,
        "500"    => fk::Weight::MEDIUM,
        "600"    => fk::Weight::SEMIBOLD,
        "700"    => fk::Weight::BOLD,
        "800"    => fk::Weight::EXTRA_BOLD,
        "900"    => fk::Weight::BLACK,
        "bolder" | "lighter" => {
            warn!("'bolder' and 'lighter' font-weight must be already resolved.");
            fk::Weight::NORMAL
        }
        _ => fk::Weight::NORMAL,
    };

    let stretch = attrs.get_str_or(AId::FontStretch, "normal");
    let stretch = match stretch {
        "normal"                 => fk::Stretch::NORMAL,
        "ultra-condensed"        => fk::Stretch::ULTRA_CONDENSED,
        "extra-condensed"        => fk::Stretch::EXTRA_CONDENSED,
        "narrower" | "condensed" => fk::Stretch::CONDENSED,
        "semi-condensed"         => fk::Stretch::SEMI_CONDENSED,
        "semi-expanded"          => fk::Stretch::SEMI_EXPANDED,
        "wider" | "expanded"     => fk::Stretch::EXPANDED,
        "extra-expanded"         => fk::Stretch::EXTRA_EXPANDED,
        "ultra-expanded"         => fk::Stretch::ULTRA_EXPANDED,
        _                        => fk::Stretch::NORMAL,
    };

    let mut font_list = Vec::new();
    let font_family = attrs.get_str_or(AId::FontFamily, "");
    for family in font_family.split(',') {
        let family = family.replace('\'', "");

        let name = match family.as_ref() {
            "serif"      => fk::FamilyName::Serif,
            "sans-serif" => fk::FamilyName::SansSerif,
            "monospace"  => fk::FamilyName::Monospace,
            "cursive"    => fk::FamilyName::Cursive,
            "fantasy"    => fk::FamilyName::Fantasy,
            _            => fk::FamilyName::Title(family)
        };

        font_list.push(name);
    }

    let size = attrs.get_number_or(AId::FontSize, 0.0);
    if !(size > 0.0) {
        return None;
    }

    let letter_spacing = attrs.get_number_or(AId::LetterSpacing, 0.0);
    let word_spacing = attrs.get_number_or(AId::WordSpacing, 0.0);

    let properties = fk::Properties { style, weight, stretch };
    let handle = match fk::SystemSource::new().select_best_match(&font_list, &properties) {
        Ok(v) => v,
        Err(_) => {
            // TODO: Select any font.
            warn!("No match for {:?} font-family.", font_family);
            return None;
        }
    };

    // TODO: font caching
    let font = match handle.load() {
        Ok(v) => v,
        Err(_) => {
            warn!("Failed to load font for {:?} font-family.", font_family);
            return None;
        }
    };

    let metrics = font.metrics();
    let scale = size / metrics.units_per_em as f64;

    Some(Font {
        font,
        size,
        units_per_em: metrics.units_per_em,
        ascent: metrics.ascent as f64 * scale,
        underline_position: metrics.underline_position as f64 * scale,
        underline_thickness: metrics.underline_thickness as f64 * scale,
        letter_spacing,
        word_spacing,
    })
}

pub fn resolve_text_anchor(node: &svgdom::Node) -> TextAnchor {
    let attrs = node.attributes();
    match attrs.get_str_or(AId::TextAnchor, "start") {
        "start"  => TextAnchor::Start,
        "middle" => TextAnchor::Middle,
        "end"    => TextAnchor::End,
        _        => TextAnchor::Start,
    }
}

// According to the https://github.com/w3c/svgwg/issues/537
// 'Assignment of multi-value text layout attributes (x, y, dx, dy, rotate) should be
// according to Unicode code point characters.'
pub fn resolve_positions_list(text_elem: &svgdom::Node) -> PositionsList {
    let total = count_chars(text_elem);

    let mut list = vec![CharacterPosition {
        x: None,
        y: None,
        dx: None,
        dy: None,
    }; total];

    let mut offset = 0;
    for child in text_elem.descendants() {
        if child.is_element() {
            let total = count_chars(&child);
            let ref attrs = child.attributes();

            macro_rules! push_list {
                ($aid:expr, $field:ident) => {
                    if let Some(num_list) = attrs.get_number_list($aid) {
                        let len = cmp::min(num_list.len(), total);
                        for i in 0..len {
                            list[offset + i].$field = Some(num_list[i]);
                        }
                    }
                };
            }

            push_list!(AId::X, x);
            push_list!(AId::Y, y);
            push_list!(AId::Dx, dx);
            push_list!(AId::Dy, dy);
        } else {
            offset += child.text().chars().count();
        }
    }

    list
}

// TODO: simplify
pub fn resolve_rotate(parent: &svgdom::Node, mut offset: usize, list: &mut RotateList) {
    for child in parent.children() {
        if child.is_text() {
            let chars_count = child.text().chars().count();
            // TODO: should stop at the root 'text'
            if let Some(p) = child.find_node_with_attribute(AId::Rotate) {
                let attrs = p.attributes();
                if let Some(rotate_list) = attrs.get_number_list(AId::Rotate) {
                    for i in 0..chars_count {
                        let r = match rotate_list.get(i + offset) {
                            Some(r) => *r,
                            None => {
                                // Use the last angle if the index is out of bounds.
                                *rotate_list.last().unwrap_or(&0.0)
                            }
                        };

                        list.push(r);
                    }

                    offset += chars_count;
                }
            } else {
                for _ in 0..chars_count {
                    list.push(0.0);
                }
            }
        } else if child.is_element() {
            // Use parent rotate list if it is not set.
            let sub_offset = if child.has_attribute(AId::Rotate) { 0 } else { offset };
            resolve_rotate(&child, sub_offset, list);

            // TODO: why?
            // 'tspan' represents a single char.
            offset += 1;
        }
    }
}

fn count_chars(node: &svgdom::Node) -> usize {
    let mut total = 0;
    for child in node.descendants().filter(|n| n.is_text()) {
        total += child.text().chars().count();
    }

    total
}


#[derive(Clone, Debug)]
pub struct TextDecorationStyle {
    pub fill: Option<tree::Fill>,
    pub stroke: Option<tree::Stroke>,
}

#[derive(Clone, Debug)]
pub struct TextDecoration {
    pub underline: Option<TextDecorationStyle>,
    pub overline: Option<TextDecorationStyle>,
    pub line_through: Option<TextDecorationStyle>,
}

pub fn resolve_decoration(
    tree: &tree::Tree,
    node: &svgdom::Node,
    tspan: &svgdom::Node
) -> TextDecoration {
    let text_dec = conv_text_decoration(node);
    let tspan_dec = conv_tspan_decoration(tspan);

    let gen_style = |in_tspan: bool, in_text: bool| {
        let n = if in_tspan {
            tspan.clone()
        } else if in_text {
            node.clone()
        } else {
            return None;
        };

        let ref attrs = n.attributes();
        let fill = super::super::fill::convert(tree, attrs, true);
        let stroke = super::super::stroke::convert(tree, attrs, true);

        Some(TextDecorationStyle {
            fill,
            stroke,
        })
    };

    TextDecoration {
        underline: gen_style(tspan_dec.has_underline, text_dec.has_underline),
        overline: gen_style(tspan_dec.has_overline, text_dec.has_overline),
        line_through: gen_style(tspan_dec.has_line_through, text_dec.has_line_through),
    }
}

struct TextDecoTypes {
    has_underline: bool,
    has_overline: bool,
    has_line_through: bool,
}

// 'text-decoration' defined in the 'text' element
// should be generated by 'prepare_text_decoration'.
fn conv_text_decoration(node: &svgdom::Node) -> TextDecoTypes {
    debug_assert!(node.is_tag_name(EId::Text));

    let attrs = node.attributes();

    let text = attrs.get_str_or(AId::TextDecoration, "");

    TextDecoTypes {
        has_underline: text.contains("underline"),
        has_overline: text.contains("overline"),
        has_line_through: text.contains("line-through"),
    }
}

// 'text-decoration' in 'tspan' does not depend on parent elements.
fn conv_tspan_decoration(tspan: &svgdom::Node) -> TextDecoTypes {
    let attrs = tspan.attributes();

    let has_attr = |decoration_id: &str| {
        if let Some(id) = attrs.get_str(AId::TextDecoration) {
            if id == decoration_id {
                return true;
            }
        }

        false
    };

    TextDecoTypes {
        has_underline: has_attr("underline"),
        has_overline: has_attr("overline"),
        has_line_through: has_attr("line-through"),
    }
}
