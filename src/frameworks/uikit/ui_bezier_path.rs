/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIBezierPath`.

use crate::frameworks::core_graphics::cg_affine_transform::CGAffineTransform;
use crate::frameworks::core_graphics::{CGRect, CGFloat};
use crate::objc::{id, msg, objc_classes, ClassExports, HostObject, NSZonePtr};
use crate::Environment;

#[derive(Default)]
struct UIBezierPathHostObject {
    bounds: CGRect,
    corner_radius: CGFloat,
    uses_even_odd_fill_rule: bool,
}
impl HostObject for UIBezierPathHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIBezierPath: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc
        .alloc_object(this, Box::<UIBezierPathHostObject>::default(), &mut env.mem)
}

+ (id)bezierPathWithRoundedRect:(CGRect)rect cornerRadius:(CGFloat)corner_radius {
    let path: id = msg![env; this alloc];
    let host_object = env.objc.borrow_mut::<UIBezierPathHostObject>(path);
    host_object.bounds = rect;
    host_object.corner_radius = corner_radius.max(0.0).min(rect.size.width / 2.0).min(rect.size.height / 2.0);
    path
}

+ (id)bezierPathWithRect:(CGRect)rect {
    let path: id = msg![env; this alloc];
    env.objc.borrow_mut::<UIBezierPathHostObject>(path).bounds = rect;
    path
}

- (CGRect)bounds {
    env.objc.borrow::<UIBezierPathHostObject>(this).bounds
}

- (())applyTransform:(CGAffineTransform)transform {
    let host_object = env.objc.borrow_mut::<UIBezierPathHostObject>(this);
    host_object.bounds = transform.apply_to_rect(host_object.bounds);
}

- (())appendPath:(id)path {
    let other = env.objc.borrow::<UIBezierPathHostObject>(path);
    let other_bounds = other.bounds;
    let other_corner_radius = other.corner_radius;
    let host_object = env.objc.borrow_mut::<UIBezierPathHostObject>(this);
    let bounds = host_object.bounds;
    if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
        host_object.bounds = other_bounds;
    } else if other_bounds.size.width > 0.0 && other_bounds.size.height > 0.0 {
        let min_x = bounds.origin.x.min(other_bounds.origin.x);
        let min_y = bounds.origin.y.min(other_bounds.origin.y);
        let max_x = (bounds.origin.x + bounds.size.width)
            .max(other_bounds.origin.x + other_bounds.size.width);
        let max_y = (bounds.origin.y + bounds.size.height)
            .max(other_bounds.origin.y + other_bounds.size.height);
        host_object.bounds = CGRect {
            origin: crate::frameworks::core_graphics::CGPoint { x: min_x, y: min_y },
            size: crate::frameworks::core_graphics::CGSize {
                width: max_x - min_x,
                height: max_y - min_y,
            },
        };
    }
    host_object.corner_radius = host_object.corner_radius.max(other_corner_radius);
}

- (CGFloat)lineWidth {
    1.0
}

- (bool)usesEvenOddFillRule {
    env.objc
        .borrow::<UIBezierPathHostObject>(this)
        .uses_even_odd_fill_rule
}

- (())setUsesEvenOddFillRule:(bool)value {
    env.objc
        .borrow_mut::<UIBezierPathHostObject>(this)
        .uses_even_odd_fill_rule = value;
}

- (())fill {
    log!("TODO: [(UIBezierPath*){:?} fill] rounded path rendering", this);
}

- (())stroke {
    log!("TODO: [(UIBezierPath*){:?} stroke] rounded path rendering", this);
}

- (())addClip {
    log!("TODO: [(UIBezierPath*){:?} addClip] rounded path clipping", this);
}

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};
