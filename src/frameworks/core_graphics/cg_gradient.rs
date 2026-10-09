/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Core Graphics gradients.

use super::cg_color::CGColorHostObject;
use super::cg_color_space::{CGColorSpaceHostObject, CGColorSpaceRef};
use super::cg_context::CGContextRef;
use super::{CGPoint, CGFloat};
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::{CFRelease, CFTypeRef};
use crate::mem::MutPtr;
use crate::objc::{id, msg, objc_classes, ClassExports, HostObject, ObjC};
use crate::Environment;

#[derive(Clone, Copy)]
pub(super) struct GradientStop {
    pub(super) location: CGFloat,
    pub(super) color: (CGFloat, CGFloat, CGFloat, CGFloat),
}

#[derive(Default)]
struct CGGradientHostObject {
    stops: Vec<GradientStop>,
}
impl HostObject for CGGradientHostObject {}

pub type CGGradientRef = CFTypeRef;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_CGGradient: NSObject
@end

};

fn CGGradientCreateWithColors(
    env: &mut Environment,
    color_space: CGColorSpaceRef,
    colors: id,
    locations: MutPtr<CGFloat>,
) -> CGGradientRef {
    assert_eq!(
        env.objc.borrow::<CGColorSpaceHostObject>(color_space).name,
        super::cg_color_space::kCGColorSpaceGenericRGB
    );
    let count: u32 = msg![env; colors count];
    assert!(count > 0);
    let mut stops = Vec::with_capacity(count as usize);
    for index in 0..count {
        let color: id = msg![env; colors objectAtIndex:index];
        let color = env.objc.borrow::<CGColorHostObject>(color);
        let location = if locations.is_null() {
            index as CGFloat / (count - 1).max(1) as CGFloat
        } else {
            env.mem.read(locations + index)
        };
        stops.push(GradientStop {
            location,
            color: (color.r, color.g, color.b, color.a),
        });
    }
    let class = env
        .objc
        .get_known_class("_touchHLE_CGGradient", &mut env.mem);
    env.objc
        .alloc_object(class, Box::new(CGGradientHostObject { stops }), &mut env.mem)
}

fn CGGradientRelease(env: &mut Environment, gradient: CGGradientRef) {
    if !gradient.is_null() {
        CFRelease(env, gradient);
    }
}

fn CGGradientGetStops(objc: &ObjC, gradient: CGGradientRef) -> Vec<GradientStop> {
    objc.borrow::<CGGradientHostObject>(gradient).stops.clone()
}

pub(super) fn interpolate(
    stops: &[GradientStop],
    location: CGFloat,
) -> (CGFloat, CGFloat, CGFloat, CGFloat) {
    let Some(first) = stops.first() else {
        return (0.0, 0.0, 0.0, 0.0);
    };
    if location <= first.location {
        return first.color;
    }
    for pair in stops.windows(2) {
        let (left, right) = (pair[0], pair[1]);
        if location <= right.location {
            let span = right.location - left.location;
            let t = if span == 0.0 {
                1.0
            } else {
                (location - left.location) / span
            };
            return (
                left.color.0 + (right.color.0 - left.color.0) * t,
                left.color.1 + (right.color.1 - left.color.1) * t,
                left.color.2 + (right.color.2 - left.color.2) * t,
                left.color.3 + (right.color.3 - left.color.3) * t,
            );
        }
    }
    stops.last().unwrap().color
}

pub fn draw_linear_gradient(
    objc: &ObjC,
    mem: &mut crate::mem::Mem,
    context: CGContextRef,
    start: CGPoint,
    end: CGPoint,
    gradient: CGGradientRef,
) {
    let stops = CGGradientGetStops(objc, gradient);
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared == 0.0 {
        return;
    }
    let mut drawer = super::cg_bitmap_context::CGBitmapContextDrawer::new(objc, mem, context);
    let width = drawer.width() as i32;
    let height = drawer.height() as i32;
    let rect = super::CGRect {
        origin: super::CGPoint { x: 0.0, y: 0.0 },
        size: super::CGSize {
            width: width as CGFloat,
            height: height as CGFloat,
        },
    };
    let samples: Vec<_> = drawer
        .iter_transformed_pixels(rect)
        .map(|(coords, (x, y))| {
            let px = x * width as CGFloat;
            let py = y * height as CGFloat;
            let t = ((px - start.x) * dx + (py - start.y) * dy) / length_squared;
            (coords, interpolate(&stops, t.clamp(0.0, 1.0)))
        })
        .collect();
    for (coords, color) in samples {
        drawer.put_pixel(coords, color, true);
    }
}

fn CGContextDrawLinearGradient(
    env: &mut Environment,
    context: CGContextRef,
    gradient: CGGradientRef,
    start: CGPoint,
    end: CGPoint,
    _options: u32,
) {
    draw_linear_gradient(&env.objc, &mut env.mem, context, start, end, gradient);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGGradientCreateWithColors(_, _, _)),
    export_c_func!(CGGradientRelease(_)),
    export_c_func!(CGContextDrawLinearGradient(_, _, _, _, _)),
];
