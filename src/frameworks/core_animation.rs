/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Core Animation framework.
//!
//! Useful resources:
//! - Apple's [Core Animation Programming Guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/CoreAnimation_guide/Introduction/Introduction.html)

pub mod ca_animation;
pub mod ca_display_link;
pub mod ca_eagl_layer;
pub mod ca_layer;
pub mod ca_media_timing_function;
pub mod ca_transaction;

mod animation;
mod composition;

pub use composition::recomposite_if_necessary;

use crate::abi::{impl_GuestRet_for_large_struct, GuestArg};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::core_graphics::CGFloat;
use crate::mem::SafeRead;
use crate::Environment;
use std::time::Instant;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct CATransform3D {
    pub m11: CGFloat,
    pub m12: CGFloat,
    pub m13: CGFloat,
    pub m14: CGFloat,
    pub m21: CGFloat,
    pub m22: CGFloat,
    pub m23: CGFloat,
    pub m24: CGFloat,
    pub m31: CGFloat,
    pub m32: CGFloat,
    pub m33: CGFloat,
    pub m34: CGFloat,
    pub m41: CGFloat,
    pub m42: CGFloat,
    pub m43: CGFloat,
    pub m44: CGFloat,
}
unsafe impl SafeRead for CATransform3D {}
impl_GuestRet_for_large_struct!(CATransform3D);

impl GuestArg for CATransform3D {
    const REG_COUNT: usize = 16;

    fn from_regs(regs: &[u32]) -> Self {
        Self {
            m11: GuestArg::from_regs(&regs[0..1]),
            m12: GuestArg::from_regs(&regs[1..2]),
            m13: GuestArg::from_regs(&regs[2..3]),
            m14: GuestArg::from_regs(&regs[3..4]),
            m21: GuestArg::from_regs(&regs[4..5]),
            m22: GuestArg::from_regs(&regs[5..6]),
            m23: GuestArg::from_regs(&regs[6..7]),
            m24: GuestArg::from_regs(&regs[7..8]),
            m31: GuestArg::from_regs(&regs[8..9]),
            m32: GuestArg::from_regs(&regs[9..10]),
            m33: GuestArg::from_regs(&regs[10..11]),
            m34: GuestArg::from_regs(&regs[11..12]),
            m41: GuestArg::from_regs(&regs[12..13]),
            m42: GuestArg::from_regs(&regs[13..14]),
            m43: GuestArg::from_regs(&regs[14..15]),
            m44: GuestArg::from_regs(&regs[15..16]),
        }
    }

    fn to_regs(self, regs: &mut [u32]) {
        self.m11.to_regs(&mut regs[0..1]);
        self.m12.to_regs(&mut regs[1..2]);
        self.m13.to_regs(&mut regs[2..3]);
        self.m14.to_regs(&mut regs[3..4]);
        self.m21.to_regs(&mut regs[4..5]);
        self.m22.to_regs(&mut regs[5..6]);
        self.m23.to_regs(&mut regs[6..7]);
        self.m24.to_regs(&mut regs[7..8]);
        self.m31.to_regs(&mut regs[8..9]);
        self.m32.to_regs(&mut regs[9..10]);
        self.m33.to_regs(&mut regs[10..11]);
        self.m34.to_regs(&mut regs[11..12]);
        self.m41.to_regs(&mut regs[12..13]);
        self.m42.to_regs(&mut regs[13..14]);
        self.m43.to_regs(&mut regs[14..15]);
        self.m44.to_regs(&mut regs[15..16]);
    }
}

const CATransform3D_IDENTITY: CATransform3D = CATransform3D {
    m11: 1.0,
    m12: 0.0,
    m13: 0.0,
    m14: 0.0,
    m21: 0.0,
    m22: 1.0,
    m23: 0.0,
    m24: 0.0,
    m31: 0.0,
    m32: 0.0,
    m33: 1.0,
    m34: 0.0,
    m41: 0.0,
    m42: 0.0,
    m43: 0.0,
    m44: 1.0,
};

pub const CONSTANTS: ConstantExports = &[(
    "_CATransform3DIdentity",
    HostConstant::Custom(|env| {
        env.mem
            .alloc_and_write(CATransform3D_IDENTITY)
            .cast()
            .cast_const()
    }),
)];

fn CATransform3DRotate(
    _env: &mut Environment,
    transform: CATransform3D,
    angle: CGFloat,
    x: CGFloat,
    y: CGFloat,
    z: CGFloat,
) -> CATransform3D {
    let axis_length = (x * x + y * y + z * z).sqrt();
    if axis_length == 0.0 {
        return transform;
    }

    let (x, y, z) = (x / axis_length, y / axis_length, z / axis_length);
    let (sin, cos) = angle.sin_cos();
    let t = 1.0 - cos;
    let rotation = [
        [
            t * x * x + cos,
            t * x * y + sin * z,
            t * x * z - sin * y,
            0.0,
        ],
        [
            t * x * y - sin * z,
            t * y * y + cos,
            t * y * z + sin * x,
            0.0,
        ],
        [
            t * x * z + sin * y,
            t * y * z - sin * x,
            t * z * z + cos,
            0.0,
        ],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let rows = [
        [transform.m11, transform.m12, transform.m13, transform.m14],
        [transform.m21, transform.m22, transform.m23, transform.m24],
        [transform.m31, transform.m32, transform.m33, transform.m34],
        [transform.m41, transform.m42, transform.m43, transform.m44],
    ];
    let mut result = [[0.0; 4]; 4];
    for row in 0..4 {
        for column in 0..4 {
            for k in 0..4 {
                result[row][column] += rows[row][k] * rotation[k][column];
            }
        }
    }
    CATransform3D {
        m11: result[0][0],
        m12: result[0][1],
        m13: result[0][2],
        m14: result[0][3],
        m21: result[1][0],
        m22: result[1][1],
        m23: result[1][2],
        m24: result[1][3],
        m31: result[2][0],
        m32: result[2][1],
        m33: result[2][2],
        m34: result[2][3],
        m41: result[3][0],
        m42: result[3][1],
        m43: result[3][2],
        m44: result[3][3],
    }
}

pub const TRANSFORM_FUNCTIONS: FunctionExports =
    &[export_c_func!(CATransform3DRotate(_, _, _, _, _))];

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    // Core Animation is considered its own framework, but it technically lives
    // in a binary called QuartzCore, which does not contain anything else of
    // interest in iPhone OS 2 and 3. (iOS 5 adds Core Image to QuartzCore.)
    path: "/System/Library/Frameworks/QuartzCore.framework/QuartzCore",
    aliases: &[],
    class_exports: &[
        ca_animation::CLASSES,
        ca_display_link::CLASSES,
        ca_eagl_layer::CLASSES,
        ca_layer::CLASSES,
        ca_media_timing_function::CLASSES,
        ca_transaction::CLASSES,
    ],
    constant_exports: &[
        CONSTANTS,
        ca_animation::CONSTANTS,
        ca_layer::CONSTANTS,
        ca_media_timing_function::CONSTANTS,
        ca_transaction::CONSTANTS,
    ],
    function_exports: &[TRANSFORM_FUNCTIONS, FUNCTIONS],
};

#[derive(Default)]
pub struct State {
    ca_media_timing_function: ca_media_timing_function::State,
    composition: composition::State,
}

#[derive(Default)]
pub struct ThreadLocalState {
    ca_transaction: ca_transaction::ThreadLocalState,
}

// This function should call mach_absolute_time() and convert the result into
// seconds. Since in our implementation, mach_absolute_time() returns, in
// nanoseconds, Instant::now, we can just do the same in seconds and save
// the calls to the guest functions.
pub fn CACurrentMediaTime(env: &mut Environment) -> CFTimeInterval {
    Instant::now()
        .duration_since(env.startup_time)
        .as_secs_f64()
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CACurrentMediaTime())];
