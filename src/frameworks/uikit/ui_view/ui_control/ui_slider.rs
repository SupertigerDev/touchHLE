/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISlider`.

use crate::frameworks::core_graphics::CGRect;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_super, objc_classes, todo_objc_setter,
    ClassExports, NSZonePtr,
};

#[derive(Default)]
struct UISliderHostObject {
    superclass: super::UIControlHostObject,
    minimum_value: f32,
    maximum_value: f32,
    value: f32,
}
impl_HostObject_with_superclass!(UISliderHostObject);

fn clamp_value(slider: &mut UISliderHostObject) {
    slider.value = slider.value.max(slider.minimum_value).min(slider.maximum_value);
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISlider: UIControl

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(this, Box::<UISliderHostObject>::default(), &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    log!("[(UISlider*){:?} initWithFrame:{:?}] TODO: Implement UISlider. The control won't be rendered.", this, frame);
    msg_super![env; this initWithFrame:frame]
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log!("[(UISlider*){:?} initWithCoder:{:?}] TODO: Implement UISlider. The control won't be rendered.", this, coder);
    msg_super![env; this initWithCoder:coder]
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
}

- (f32)value {
    env.objc.borrow::<UISliderHostObject>(this).value
}
- (())setValue:(f32)value {
    let slider = env.objc.borrow_mut::<UISliderHostObject>(this);
    slider.value = value;
    clamp_value(slider);
}
- (())setValue:(f32)value animated:(bool)_animated {
    () = msg![env; this setValue:value];
}

- (())setMinimumValueImage:(id)img { // UIImage *
    todo_objc_setter!(this, img);
}
- (())setMaximumValueImage:(id)img { // UIImage *
    todo_objc_setter!(this, img);
}

// TODO: all of it

@end

};
