/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISlider`.

use super::{send_actions, UIControlEventValueChanged};
use crate::environment::Environment;
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    ClassExports, NSZonePtr,
};

const TRACK_HEIGHT: f32 = 4.0;
const THUMB_SIZE: f32 = 20.0;
const IMAGE_GAP: f32 = 5.0;

#[derive(Default)]
struct UISliderHostObject {
    superclass: super::UIControlHostObject,
    minimum_value: f32,
    maximum_value: f32,
    value: f32,
    track: id,
    filled_track: id,
    thumb: id,
    minimum_value_image_view: id,
    maximum_value_image_view: id,
}
impl_HostObject_with_superclass!(UISliderHostObject);

fn clamp_value(slider: &mut UISliderHostObject) {
    slider.value = slider
        .value
        .max(slider.minimum_value)
        .min(slider.maximum_value);
}

fn image_size(env: &mut Environment, image_view: id) -> CGSize {
    let image: id = msg![env; image_view image];
    if image == nil {
        CGSize {
            width: 0.0,
            height: 0.0,
        }
    } else {
        msg![env; image size]
    }
}

fn track_limits(
    bounds: CGRect,
    minimum_image_size: CGSize,
    maximum_image_size: CGSize,
) -> (f32, f32) {
    let left_inset = (THUMB_SIZE / 2.0).max(minimum_image_size.width + IMAGE_GAP);
    let right_inset = (THUMB_SIZE / 2.0).max(maximum_image_size.width + IMAGE_GAP);
    (
        bounds.origin.x + left_inset,
        bounds.origin.x + bounds.size.width - right_inset,
    )
}

fn layout(env: &mut Environment, this: id) {
    let bounds: CGRect = msg![env; this bounds];
    let (
        track,
        filled_track,
        thumb,
        minimum_image_view,
        maximum_image_view,
        value,
        minimum,
        maximum,
    ) = {
        let slider = env.objc.borrow::<UISliderHostObject>(this);
        (
            slider.track,
            slider.filled_track,
            slider.thumb,
            slider.minimum_value_image_view,
            slider.maximum_value_image_view,
            slider.value,
            slider.minimum_value,
            slider.maximum_value,
        )
    };
    let minimum_image_size = image_size(env, minimum_image_view);
    let maximum_image_size = image_size(env, maximum_image_view);
    let (track_left, track_right) = track_limits(bounds, minimum_image_size, maximum_image_size);
    let track_width = (track_right - track_left).max(0.0);
    let fraction = if maximum > minimum {
        ((value - minimum) / (maximum - minimum)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let thumb_center = track_left + track_width * fraction;
    let center_y = bounds.origin.y + bounds.size.height / 2.0;

    let track_frame = CGRect {
        origin: CGPoint {
            x: track_left,
            y: center_y - TRACK_HEIGHT / 2.0,
        },
        size: CGSize {
            width: track_width,
            height: TRACK_HEIGHT,
        },
    };
    () = msg![env; track setFrame:track_frame];
    () = msg![env; filled_track setFrame:(CGRect {
        origin: track_frame.origin,
        size: CGSize {
            width: (thumb_center - track_left).max(0.0),
            height: TRACK_HEIGHT,
        },
    })];
    () = msg![env; thumb setFrame:(CGRect {
        origin: CGPoint {
            x: thumb_center - THUMB_SIZE / 2.0,
            y: center_y - THUMB_SIZE / 2.0,
        },
        size: CGSize {
            width: THUMB_SIZE,
            height: THUMB_SIZE,
        },
    })];
    () = msg![env; minimum_image_view setFrame:(CGRect {
        origin: CGPoint {
            x: bounds.origin.x,
            y: center_y - minimum_image_size.height / 2.0,
        },
        size: minimum_image_size,
    })];
    () = msg![env; maximum_image_view setFrame:(CGRect {
        origin: CGPoint {
            x: bounds.origin.x + bounds.size.width - maximum_image_size.width,
            y: center_y - maximum_image_size.height / 2.0,
        },
        size: maximum_image_size,
    })];
}

fn set_value_from_touch(env: &mut Environment, this: id, touch: id) -> bool {
    let point: CGPoint = msg![env; touch locationInView:this];
    let bounds: CGRect = msg![env; this bounds];
    let (minimum_image_view, maximum_image_view, minimum, maximum, old_value) = {
        let slider = env.objc.borrow::<UISliderHostObject>(this);
        (
            slider.minimum_value_image_view,
            slider.maximum_value_image_view,
            slider.minimum_value,
            slider.maximum_value,
            slider.value,
        )
    };
    let minimum_image_size = image_size(env, minimum_image_view);
    let maximum_image_size = image_size(env, maximum_image_view);
    let (track_left, track_right) = track_limits(bounds, minimum_image_size, maximum_image_size);
    let track_width = track_right - track_left;
    if track_width <= 0.0 || maximum <= minimum {
        return false;
    }

    let fraction = ((point.x - track_left) / track_width).clamp(0.0, 1.0);
    let value = minimum + (maximum - minimum) * fraction;
    if value == old_value {
        return false;
    }
    () = msg![env; this setValue:value];
    true
}

fn init_common(env: &mut Environment, this: id) -> id {
    let track = msg_class![env; UIView new];
    let filled_track = msg_class![env; UIView new];
    let thumb = msg_class![env; UIView new];
    let minimum_value_image_view = msg_class![env; UIImageView new];
    let maximum_value_image_view = msg_class![env; UIImageView new];

    let track_color: id = msg_class![env; UIColor lightGrayColor];
    let filled_track_color: id = msg_class![env; UIColor colorWithRed:(0.0f32)
                                                                 green:(122.0f32 / 255.0)
                                                                  blue:(1.0f32)
                                                                 alpha:(1.0f32)];
    let thumb_color: id = msg_class![env; UIColor whiteColor];
    () = msg![env; track setBackgroundColor:track_color];
    () = msg![env; filled_track setBackgroundColor:filled_track_color];
    () = msg![env; thumb setBackgroundColor:thumb_color];
    () = msg![env; track setUserInteractionEnabled:false];
    () = msg![env; filled_track setUserInteractionEnabled:false];
    () = msg![env; thumb setUserInteractionEnabled:false];
    () = msg![env; minimum_value_image_view setUserInteractionEnabled:false];
    () = msg![env; maximum_value_image_view setUserInteractionEnabled:false];

    let host_object = env.objc.borrow_mut::<UISliderHostObject>(this);
    host_object.maximum_value = 1.0;
    host_object.track = track;
    host_object.filled_track = filled_track;
    host_object.thumb = thumb;
    host_object.minimum_value_image_view = minimum_value_image_view;
    host_object.maximum_value_image_view = maximum_value_image_view;

    () = msg![env; this addSubview:track];
    () = msg![env; this addSubview:filled_track];
    () = msg![env; this addSubview:minimum_value_image_view];
    () = msg![env; this addSubview:maximum_value_image_view];
    () = msg![env; this addSubview:thumb];

    let track_layer: id = msg![env; track layer];
    () = msg![env; track_layer setCornerRadius:(TRACK_HEIGHT / 2.0)];
    let filled_track_layer: id = msg![env; filled_track layer];
    () = msg![env; filled_track_layer setCornerRadius:(TRACK_HEIGHT / 2.0)];
    let thumb_layer: id = msg![env; thumb layer];
    () = msg![env; thumb_layer setCornerRadius:(THUMB_SIZE / 2.0)];
    () = msg![env; this setOpaque:false];

    layout(env, this);
    this
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISlider: UIControl

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(this, Box::<UISliderHostObject>::default(), &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    init_common(env, this)
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    init_common(env, this)
}

- (())dealloc {
    let UISliderHostObject {
        superclass: _,
        minimum_value: _,
        maximum_value: _,
        value: _,
        track,
        filled_track,
        thumb,
        minimum_value_image_view,
        maximum_value_image_view,
    } = std::mem::take(env.objc.borrow_mut(this));
    release(env, track);
    release(env, filled_track);
    release(env, thumb);
    release(env, minimum_value_image_view);
    release(env, maximum_value_image_view);
    msg_super![env; this dealloc]
}

- (f32)minimumValue {
    env.objc.borrow::<UISliderHostObject>(this).minimum_value
}
- (())setMinimumValue:(f32)value {
    let slider = env.objc.borrow_mut::<UISliderHostObject>(this);
    slider.minimum_value = value;
    if slider.maximum_value < value {
        slider.maximum_value = value;
    }
    clamp_value(slider);
    layout(env, this);
}

- (f32)maximumValue {
    env.objc.borrow::<UISliderHostObject>(this).maximum_value
}
- (())setMaximumValue:(f32)value {
    let slider = env.objc.borrow_mut::<UISliderHostObject>(this);
    slider.maximum_value = value;
    if slider.minimum_value > value {
        slider.minimum_value = value;
    }
    clamp_value(slider);
    layout(env, this);
}

- (f32)value {
    env.objc.borrow::<UISliderHostObject>(this).value
}
- (())setValue:(f32)value {
    let slider = env.objc.borrow_mut::<UISliderHostObject>(this);
    slider.value = value;
    clamp_value(slider);
    layout(env, this);
}
- (())setValue:(f32)value animated:(bool)_animated {
    () = msg![env; this setValue:value];
}

- (())setMinimumValueImage:(id)image { // UIImage *
    let image_view = env.objc.borrow::<UISliderHostObject>(this).minimum_value_image_view;
    () = msg![env; image_view setImage:image];
    layout(env, this);
}
- (id)minimumValueImage {
    let image_view = env.objc.borrow::<UISliderHostObject>(this).minimum_value_image_view;
    msg![env; image_view image]
}
- (())setMaximumValueImage:(id)image { // UIImage *
    let image_view = env.objc.borrow::<UISliderHostObject>(this).maximum_value_image_view;
    () = msg![env; image_view setImage:image];
    layout(env, this);
}
- (id)maximumValueImage {
    let image_view = env.objc.borrow::<UISliderHostObject>(this).maximum_value_image_view;
    msg![env; image_view image]
}

- (())layoutSubviews {
    layout(env, this);
    msg_super![env; this layoutSubviews]
}

- (bool)beginTrackingWithTouch:(id)touch withEvent:(id)event {
    if set_value_from_touch(env, this, touch) {
        send_actions(env, this, event, UIControlEventValueChanged);
    }
    true
}
- (bool)continueTrackingWithTouch:(id)touch withEvent:(id)event {
    if set_value_from_touch(env, this, touch) {
        send_actions(env, this, event, UIControlEventValueChanged);
    }
    true
}
- (())endTrackingWithTouch:(id)touch withEvent:(id)event {
    if set_value_from_touch(env, this, touch) {
        send_actions(env, this, event, UIControlEventValueChanged);
    }
    msg_super![env; this endTrackingWithTouch:touch withEvent:event]
}

- (id)hitTest:(CGPoint)point withEvent:(id)event {
    if msg![env; this pointInside:point withEvent:event] {
        this
    } else {
        nil
    }
}

@end

};
