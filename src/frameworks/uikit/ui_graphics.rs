/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIGraphics.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_graphics::cg_bitmap_context::{
    CGBitmapContextCreate, CGBitmapContextCreateImage,
};
use crate::frameworks::core_graphics::cg_color_space::{
    CGColorSpaceCreateDeviceRGB, CGColorSpaceRelease,
};
use crate::frameworks::core_graphics::cg_context::CGContextScaleCTM;
use crate::frameworks::core_graphics::cg_context::{
    CGContextRef, CGContextRelease, CGContextRetain,
};
use crate::frameworks::core_graphics::cg_image::{
    kCGImageAlphaNoneSkipLast, kCGImageAlphaPremultipliedLast, CGImageRelease,
};
use crate::frameworks::core_graphics::{CGFloat, CGSize};
use crate::mem::{GuestUSize, MutVoidPtr};
use crate::objc::{msg_class, nil};
use crate::Environment;

#[derive(Default)]
pub(super) struct State {
    pub(super) context_stack: Vec<CGContextRef>,
}

pub fn UIGraphicsPushContext(env: &mut Environment, context: CGContextRef) {
    CGContextRetain(env, context);
    env.framework_state
        .uikit
        .ui_graphics
        .context_stack
        .push(context);
}
pub fn UIGraphicsPopContext(env: &mut Environment) {
    let context = env.framework_state.uikit.ui_graphics.context_stack.pop();
    CGContextRelease(env, context.unwrap());
}
pub fn UIGraphicsGetCurrentContext(env: &mut Environment) -> CGContextRef {
    env.framework_state
        .uikit
        .ui_graphics
        .context_stack
        .last()
        .copied()
        .unwrap_or(nil)
}

fn begin_image_context(env: &mut Environment, size: CGSize, opaque: bool, scale: CGFloat) {
    assert!(size.width.is_finite() && size.height.is_finite());
    assert!(size.width >= 0.0 && size.height >= 0.0);
    assert!(scale.is_finite() && scale >= 0.0);
    let scale = if scale == 0.0 { 1.0 } else { scale };
    let width = ((size.width * scale).ceil() as GuestUSize).max(1);
    let height = ((size.height * scale).ceil() as GuestUSize).max(1);
    let color_space = CGColorSpaceCreateDeviceRGB(env);
    let alpha_info = if opaque {
        kCGImageAlphaNoneSkipLast
    } else {
        kCGImageAlphaPremultipliedLast
    };
    let context = CGBitmapContextCreate(
        env,
        MutVoidPtr::null(),
        width,
        height,
        8,
        width * 4,
        color_space,
        alpha_info,
    );
    CGColorSpaceRelease(env, color_space);
    CGContextScaleCTM(env, context, scale, scale);
    UIGraphicsPushContext(env, context);
    CGContextRelease(env, context);
}

fn UIGraphicsBeginImageContextWithOptions(
    env: &mut Environment,
    size: CGSize,
    opaque: bool,
    scale: CGFloat,
) {
    begin_image_context(env, size, opaque, scale)
}

fn UIGraphicsBeginImageContext(env: &mut Environment, size: CGSize) {
    begin_image_context(env, size, false, 1.0)
}

fn UIGraphicsGetImageFromCurrentImageContext(env: &mut Environment) -> crate::objc::id {
    let context = UIGraphicsGetCurrentContext(env);
    if context == nil {
        return nil;
    }
    let cg_image = CGBitmapContextCreateImage(env, context);
    let image: crate::objc::id = msg_class![env; UIImage imageWithCGImage:cg_image];
    CGImageRelease(env, cg_image);
    image
}

fn UIGraphicsEndImageContext(env: &mut Environment) {
    UIGraphicsPopContext(env);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(UIGraphicsPushContext(_)),
    export_c_func!(UIGraphicsPopContext()),
    export_c_func!(UIGraphicsGetCurrentContext()),
    export_c_func!(UIGraphicsBeginImageContextWithOptions(_, _, _)),
    export_c_func!(UIGraphicsBeginImageContext(_)),
    export_c_func!(UIGraphicsGetImageFromCurrentImageContext()),
    export_c_func!(UIGraphicsEndImageContext()),
];
