// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// external
use cairo::{
    self,
    MatrixTrait,
};

// self
use super::prelude::*;
use backend_utils::mask;


pub fn apply(
    node: &usvg::Node,
    mask: &usvg::Mask,
    opt: &Options,
    bbox: Rect,
    layers: &mut CairoLayers,
    sub_cr: &cairo::Context,
) {
    let mask_surface = try_opt!(layers.get(), ());
    let mut mask_surface = mask_surface.borrow_mut();

    {
        let mask_cr = cairo::Context::new(&*mask_surface);
        mask_cr.set_matrix(sub_cr.get_matrix());

        let r = if mask.units == usvg::Units::ObjectBoundingBox {
            try_opt_warn!(mask.rect.bbox_transform(bbox), (),
                          "Mask '{}' cannot be used on a zero-sized object.", mask.id)
        } else {
            mask.rect
        };

        mask_cr.rectangle(r.x, r.y, r.width, r.height);
        mask_cr.clip();

        if mask.content_units == usvg::Units::ObjectBoundingBox {
            let m = try_opt_warn!(cairo::Matrix::from_bbox(bbox), (),
                                  "Mask '{}' cannot be used on a zero-sized object.", mask.id);
            mask_cr.transform(m);
        }

        super::render_group(node, opt, layers, &mask_cr);
    }

    {
        let mut data = try_opt_warn!(mask_surface.get_data().ok(), (),
                                     "Failed to borrow a surface for mask '{}'.", mask.id);
        mask::image_to_mask(&mut data, layers.image_size());
    }

    if let Some(ref id) = mask.mask {
        if let Some(ref mask_node) = node.tree().defs_by_id(id) {
            if let usvg::NodeKind::Mask(ref mask) = *mask_node.borrow() {
                apply(mask_node, mask, opt, bbox, layers, sub_cr);
            }
        }
    }

    sub_cr.set_matrix(cairo::Matrix::identity());
    sub_cr.set_source_surface(&*mask_surface, 0.0, 0.0);
    sub_cr.set_operator(cairo::Operator::DestIn);
    sub_cr.paint();

    // Reset operator.
    sub_cr.set_operator(cairo::Operator::Over);

    // Reset source to unborrow the `mask_surface` from the `Context`.
    sub_cr.reset_source_rgba();
}
