/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISegmentedControl`.

use super::{send_actions, UIControlEventValueChanged};
use crate::environment::Environment;
use crate::frameworks::core_graphics::cg_color;
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::frameworks::uikit::ui_font::UITextAlignmentCenter;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    retain, ClassExports, NSZonePtr,
};

const SEGMENT_CORNER_RADIUS: f32 = 5.0;
const SEGMENT_GAP: f32 = 1.0;
const CONTROL_INSET: f32 = 1.0;

struct UISegmentedControlHostObject {
    superclass: super::UIControlHostObject,
    selected_segment_index: NSInteger,
    segments: Vec<id>,
    segment_widths: Vec<Option<f32>>,
    tint_color: id,
    momentary: bool,
}
impl_HostObject_with_superclass!(UISegmentedControlHostObject);

impl Default for UISegmentedControlHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            selected_segment_index: -1,
            segments: Vec::new(),
            segment_widths: Vec::new(),
            tint_color: nil,
            momentary: false,
        }
    }
}

struct UISegmentHostObject {
    superclass: super::UIControlHostObject,
    title: id,
    image: id,
    label: id,
    image_view: id,
    tint_color: id,
}
impl_HostObject_with_superclass!(UISegmentHostObject);

impl Default for UISegmentHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            title: nil,
            image: nil,
            label: nil,
            image_view: nil,
            tint_color: nil,
        }
    }
}

fn update_segment(env: &mut Environment, segment: id) {
    let selected: bool = msg![env; segment isSelected];
    let enabled: bool = msg![env; segment isEnabled];
    let (label, tint_color) = {
        let segment = env.objc.borrow::<UISegmentHostObject>(segment);
        (segment.label, segment.tint_color)
    };
    let background_color: id = if selected {
        if tint_color == nil {
            msg_class![env; UIColor colorWithRed:(0.0f32)
                                           green:(122.0f32 / 255.0)
                                            blue:(1.0f32)
                                           alpha:(1.0f32)]
        } else {
            let cg_color = msg![env; tint_color CGColor];
            let (red, green, blue, _) = cg_color::to_rgba(&env.objc, cg_color);
            msg_class![env; UIColor colorWithRed:(red * 0.7)
                                           green:(green * 0.7)
                                            blue:(blue * 0.7)
                                           alpha:(1.0f32)]
        }
    } else {
        msg_class![env; UIColor colorWithRed:(0.97f32)
                                       green:(0.97f32)
                                        blue:(0.97f32)
                                       alpha:(1.0f32)]
    };
    let text_color: id = if selected {
        msg_class![env; UIColor whiteColor]
    } else {
        msg_class![env; UIColor blackColor]
    };
    () = msg![env; segment setBackgroundColor:background_color];
    () = msg![env; label setTextColor:text_color];
    () = msg![env; segment setAlpha:(if enabled { 1.0f32 } else { 0.5f32 })];
}

fn init_segment(env: &mut Environment, segment: id) -> id {
    let label: id = msg_class![env; UILabel new];
    () = msg![env; label setTextAlignment:UITextAlignmentCenter];
    () = msg![env; label setUserInteractionEnabled:false];
    () = msg![env; segment setUserInteractionEnabled:false];

    let image_view: id = msg_class![env; UIImageView new];
    () = msg![env; image_view setUserInteractionEnabled:false];

    {
        let host = env.objc.borrow_mut::<UISegmentHostObject>(segment);
        host.label = label;
        host.image_view = image_view;
    }
    () = msg![env; segment addSubview:label];
    () = msg![env; segment addSubview:image_view];
    let layer: id = msg![env; segment layer];
    () = msg![env; layer setCornerRadius:(SEGMENT_CORNER_RADIUS)];
    () = msg![env; segment setOpaque:false];
    update_segment(env, segment);
    segment
}

fn layout_segment(env: &mut Environment, segment: id) {
    let bounds: CGRect = msg![env; segment bounds];
    let (label, image_view, image) = {
        let segment = env.objc.borrow::<UISegmentHostObject>(segment);
        (segment.label, segment.image_view, segment.image)
    };
    if image == nil {
        () = msg![env; label setFrame:bounds];
        () = msg![env; image_view setFrame:(CGRect::default())];
    } else {
        let image_size: CGSize = msg![env; image size];
        let origin = CGPoint {
            x: (bounds.size.width - image_size.width) / 2.0,
            y: (bounds.size.height - image_size.height) / 2.0,
        };
        () = msg![env; image_view setFrame:(CGRect {
            origin,
            size: image_size,
        })];
        () = msg![env; label setFrame:(CGRect::default())];
    }
}

fn layout_control(env: &mut Environment, control: id) {
    let bounds: CGRect = msg![env; control bounds];
    let (segments, widths) = {
        let host = env.objc.borrow::<UISegmentedControlHostObject>(control);
        (host.segments.clone(), host.segment_widths.clone())
    };
    if segments.is_empty() {
        return;
    }

    let total_weight: f32 = segments
        .iter()
        .enumerate()
        .map(|(i, segment)| {
            widths
                .get(i)
                .and_then(|width| *width)
                .unwrap_or_else(|| {
                    let segment = *segment;
                    let frame: CGRect = msg![env; segment frame];
                    frame.size.width.max(1.0)
                })
                .max(1.0)
        })
        .sum();
    let available_width = (bounds.size.width
        - 2.0 * CONTROL_INSET
        - SEGMENT_GAP * segments.len().saturating_sub(1) as f32)
        .max(0.0);
    let mut x = bounds.origin.x + CONTROL_INSET;
    for (i, segment) in segments.iter().enumerate() {
        let weight = widths
            .get(i)
            .and_then(|width| *width)
            .unwrap_or_else(|| {
                let segment = *segment;
                let frame: CGRect = msg![env; segment frame];
                frame.size.width.max(1.0)
            })
            .max(1.0);
        let width = if i + 1 == segments.len() {
            bounds.origin.x + bounds.size.width - CONTROL_INSET - x
        } else {
            available_width * weight / total_weight
        };
        let segment = *segment;
        () = msg![env; segment setFrame:(CGRect {
            origin: CGPoint { x, y: bounds.origin.y },
            size: CGSize {
                width,
                height: bounds.size.height,
            },
        })];
        x += width + SEGMENT_GAP;
    }
}

fn set_selected_segment(env: &mut Environment, control: id, index: NSInteger) {
    let (segments, selected_index) = {
        let host = env.objc.borrow_mut::<UISegmentedControlHostObject>(control);
        let valid_index = if index >= 0 && (index as usize) < host.segments.len() {
            index
        } else {
            -1
        };
        host.selected_segment_index = valid_index;
        (host.segments.clone(), valid_index)
    };
    for (i, segment) in segments.into_iter().enumerate() {
        () = msg![env; segment setSelected:(i as NSInteger == selected_index)];
    }
}

fn set_segment_title(env: &mut Environment, control: id, index: NSUInteger, title: id) {
    let segment = env
        .objc
        .borrow::<UISegmentedControlHostObject>(control)
        .segments
        .get(index as usize)
        .copied()
        .unwrap_or(nil);
    if segment != nil {
        () = msg![env; segment setTitle:title];
    }
}

fn set_segment_image(env: &mut Environment, control: id, index: NSUInteger, image: id) {
    let segment = env
        .objc
        .borrow::<UISegmentedControlHostObject>(control)
        .segments
        .get(index as usize)
        .copied()
        .unwrap_or(nil);
    if segment != nil {
        () = msg![env; segment setImage:image];
        layout_segment(env, segment);
    }
}

fn insert_segment(env: &mut Environment, control: id, title: id, image: id, index: NSUInteger) {
    let count = env
        .objc
        .borrow::<UISegmentedControlHostObject>(control)
        .segments
        .len();
    let index = (index as usize).min(count);
    let frame = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: 1.0,
            height: 1.0,
        },
    };
    let segment: id = msg_class![env; UISegment alloc];
    let segment: id = msg![env; segment initWithFrame:frame];
    () = msg![env; segment setTitle:title];
    () = msg![env; segment setImage:image];
    let tint_color = env
        .objc
        .borrow::<UISegmentedControlHostObject>(control)
        .tint_color;
    () = msg![env; segment setTintColor:tint_color];
    () = msg![env; control insertSubview:segment atIndex:(index as NSInteger)];
    {
        let host = env.objc.borrow_mut::<UISegmentedControlHostObject>(control);
        host.segments.insert(index, segment);
        host.segment_widths.insert(index, Some(1.0));
        if host.selected_segment_index >= index as NSInteger {
            host.selected_segment_index += 1;
        }
    }
    layout_control(env, control);
    let selected = env
        .objc
        .borrow::<UISegmentedControlHostObject>(control)
        .selected_segment_index;
    set_selected_segment(env, control, selected);
}

fn init_control(env: &mut Environment, control: id) -> id {
    () = msg![env; control setOpaque:false];
    let border_color: id = msg_class![env; UIColor colorWithRed:(0.35f32)
                                                           green:(0.35f32)
                                                            blue:(0.35f32)
                                                           alpha:(1.0f32)];
    () = msg![env; control setBackgroundColor:border_color];
    let layer: id = msg![env; control layer];
    () = msg![env; layer setCornerRadius:(SEGMENT_CORNER_RADIUS)];
    control
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISegmentedControl: UIControl

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(this, Box::<UISegmentedControlHostObject>::default(), &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    init_control(env, this)
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    init_control(env, this);

    let key = get_static_str(env, "UISegments");
    let segments_array: id = msg![env; coder decodeObjectForKey:key];
    let count: NSUInteger = if segments_array == nil {
        0
    } else {
        msg![env; segments_array count]
    };
    let mut segments = Vec::with_capacity(count as usize);
    let mut widths = Vec::with_capacity(count as usize);
    for i in 0..count {
        let segment: id = msg![env; segments_array objectAtIndex:i];
        let frame: CGRect = msg![env; segment frame];
        segments.push(segment);
        widths.push(Some(frame.size.width.max(1.0)));
    }
    let tint_key = get_static_str(env, "UISegmentedControlTintColor");
    let tint_color: id = msg![env; coder decodeObjectForKey:tint_key];
    retain(env, tint_color);
    {
        let host = env.objc.borrow_mut::<UISegmentedControlHostObject>(this);
        host.segments = segments;
        host.segment_widths = widths;
        host.tint_color = tint_color;
    }

    let selected_key = get_static_str(env, "UISelectedSegmentIndex");
    let selected: NSInteger = msg![env; coder decodeIntForKey:selected_key];
    set_selected_segment(env, this, selected);
    layout_control(env, this);
    this
}

- (())dealloc {
    let UISegmentedControlHostObject {
        superclass: _,
        selected_segment_index: _,
        segments: _,
        segment_widths: _,
        tint_color,
        momentary: _,
    } = std::mem::take(env.objc.borrow_mut(this));
    release(env, tint_color);
    msg_super![env; this dealloc]
}

- (NSInteger)selectedSegmentIndex {
    env.objc
        .borrow::<UISegmentedControlHostObject>(this)
        .selected_segment_index
}
- (())setSelectedSegmentIndex:(NSInteger)index {
    set_selected_segment(env, this, index);
}

- (NSUInteger)numberOfSegments {
    env.objc
        .borrow::<UISegmentedControlHostObject>(this)
        .segments
        .len() as NSUInteger
}

- (id)initWithItems:(id)items {
    let this: id = msg![env; this initWithFrame:(<CGRect as Default>::default())];
    let count: NSUInteger = msg![env; items count];
    for index in 0..count {
        let item: id = msg![env; items objectAtIndex:index];
        insert_segment(env, this, item, nil, index);
    }
    this
}

- (())setTitle:(id)title forSegmentAtIndex:(NSUInteger)index {
    set_segment_title(env, this, index, title);
}
- (id)titleForSegmentAtIndex:(NSUInteger)index {
    let segment = env
        .objc
        .borrow::<UISegmentedControlHostObject>(this)
        .segments
        .get(index as usize)
        .copied()
        .unwrap_or(nil);
    if segment == nil {
        nil
    } else {
        msg![env; segment title]
    }
}
- (())setImage:(id)image forSegmentAtIndex:(NSUInteger)index {
    set_segment_image(env, this, index, image);
}
- (id)imageForSegmentAtIndex:(NSUInteger)index {
    let segment = env
        .objc
        .borrow::<UISegmentedControlHostObject>(this)
        .segments
        .get(index as usize)
        .copied()
        .unwrap_or(nil);
    if segment == nil {
        nil
    } else {
        msg![env; segment image]
    }
}

- (())insertSegmentWithTitle:(id)title atIndex:(NSUInteger)index animated:(bool)_animated {
    insert_segment(env, this, title, nil, index);
}
- (())insertSegmentWithImage:(id)image atIndex:(NSUInteger)index animated:(bool)_animated {
    insert_segment(env, this, nil, image, index);
}
- (())removeSegmentAtIndex:(NSUInteger)index animated:(bool)_animated {
    let segment = {
        let host = env.objc.borrow_mut::<UISegmentedControlHostObject>(this);
        if index as usize >= host.segments.len() {
            return;
        }
        host.segment_widths.remove(index as usize);
        host.segments.remove(index as usize)
    };
    () = msg![env; segment removeFromSuperview];
    let (selected, count) = {
        let host = env.objc.borrow::<UISegmentedControlHostObject>(this);
        (host.selected_segment_index, host.segments.len())
    };
    let selected = if selected == index as NSInteger {
        -1
    } else if selected > index as NSInteger {
        selected - 1
    } else if selected >= count as NSInteger {
        count as NSInteger - 1
    } else {
        selected
    };
    set_selected_segment(env, this, selected);
    layout_control(env, this);
}
- (())removeAllSegments {
    let segments = {
        let host = env.objc.borrow_mut::<UISegmentedControlHostObject>(this);
        host.selected_segment_index = -1;
        host.segment_widths.clear();
        std::mem::take(&mut host.segments)
    };
    for segment in segments {
        () = msg![env; segment removeFromSuperview];
    }
}

- (())setEnabled:(bool)enabled forSegmentAtIndex:(NSUInteger)index {
    let segment = env
        .objc
        .borrow::<UISegmentedControlHostObject>(this)
        .segments
        .get(index as usize)
        .copied()
        .unwrap_or(nil);
    if segment != nil {
        () = msg![env; segment setEnabled:enabled];
    }
}
- (bool)isEnabledForSegmentAtIndex:(NSUInteger)index {
    let segment = env
        .objc
        .borrow::<UISegmentedControlHostObject>(this)
        .segments
        .get(index as usize)
        .copied()
        .unwrap_or(nil);
    segment != nil && msg![env; segment isEnabled]
}
- (())setWidth:(f32)width forSegmentAtIndex:(NSUInteger)index {
    let host = env.objc.borrow_mut::<UISegmentedControlHostObject>(this);
    if let Some(segment_width) = host.segment_widths.get_mut(index as usize) {
        *segment_width = Some(width.max(1.0));
    }
    layout_control(env, this);
}
- (f32)widthForSegmentAtIndex:(NSUInteger)index {
    let (width, segment) = {
        let host = env.objc.borrow::<UISegmentedControlHostObject>(this);
        (
            host.segment_widths
                .get(index as usize)
                .copied()
                .flatten(),
            host.segments.get(index as usize).copied().unwrap_or(nil),
        )
    };
    if let Some(width) = width {
        width
    } else {
        let frame: CGRect = msg![env; segment frame];
        frame.size.width
    }
}
- (())setMomentary:(bool)momentary {
    env.objc
        .borrow_mut::<UISegmentedControlHostObject>(this)
        .momentary = momentary;
}
- (bool)isMomentary {
    env.objc
        .borrow::<UISegmentedControlHostObject>(this)
        .momentary
}
- (())setTintColor:(id)tint_color {
    retain(env, tint_color);
    let (old_tint, segments) = {
        let host = env.objc.borrow_mut::<UISegmentedControlHostObject>(this);
        let old = std::mem::replace(&mut host.tint_color, tint_color);
        (old, host.segments.clone())
    };
    release(env, old_tint);
    let border_color: id = if tint_color == nil {
        msg_class![env; UIColor colorWithRed:(0.35f32)
                                       green:(0.35f32)
                                        blue:(0.35f32)
                                       alpha:(1.0f32)]
    } else {
        tint_color
    };
    () = msg![env; this setBackgroundColor:border_color];
    for segment in segments {
        () = msg![env; segment setTintColor:tint_color];
    }
}
- (id)tintColor {
    env.objc
        .borrow::<UISegmentedControlHostObject>(this)
        .tint_color
}

- (())layoutSubviews {
    layout_control(env, this);
    msg_super![env; this layoutSubviews]
}

- (bool)beginTrackingWithTouch:(id)_touch withEvent:(id)_event {
    true
}
- (bool)continueTrackingWithTouch:(id)_touch withEvent:(id)_event {
    true
}
- (())endTrackingWithTouch:(id)touch withEvent:(id)event {
    let point: CGPoint = msg![env; touch locationInView:this];
    let segments = env
        .objc
        .borrow::<UISegmentedControlHostObject>(this)
        .segments
        .clone();
    let selected_index = segments.iter().position(|segment| {
        let segment = *segment;
        let frame: CGRect = msg![env; segment frame];
        point.x >= frame.origin.x
            && point.x <= frame.origin.x + frame.size.width
            && point.y >= frame.origin.y
            && point.y <= frame.origin.y + frame.size.height
    });
    if let Some(index) = selected_index {
        let segment = segments[index];
        if msg![env; segment isEnabled] {
            let old_index: NSInteger = msg![env; this selectedSegmentIndex];
            let new_index = index as NSInteger;
            if old_index != new_index {
                set_selected_segment(env, this, new_index);
                send_actions(env, this, event, UIControlEventValueChanged);
            }
        }
    }
    if msg![env; this isMomentary] {
        set_selected_segment(env, this, -1);
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

// Undocumented class used by UISegmentedControl
@implementation UISegment: UIControl

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(this, Box::<UISegmentHostObject>::default(), &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    init_segment(env, this)
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    init_segment(env, this);
    let title_key = get_static_str(env, "UISegmentInfo");
    let title: id = msg![env; coder decodeObjectForKey:title_key];
    () = msg![env; this setTitle:title];
    let tint_key = get_static_str(env, "UISegmentTintColor");
    let tint_color: id = msg![env; coder decodeObjectForKey:tint_key];
    () = msg![env; this setTintColor:tint_color];
    this
}

- (())dealloc {
    let UISegmentHostObject {
        superclass: _,
        title,
        image,
        label,
        image_view,
        tint_color,
    } = std::mem::take(env.objc.borrow_mut(this));
    release(env, title);
    release(env, image);
    release(env, label);
    release(env, image_view);
    release(env, tint_color);
    msg_super![env; this dealloc]
}

- (())setTitle:(id)title {
    retain(env, title);
    let (old_title, label) = {
        let host = env.objc.borrow_mut::<UISegmentHostObject>(this);
        (std::mem::replace(&mut host.title, title), host.label)
    };
    release(env, old_title);
    () = msg![env; label setText:title];
}
- (id)title {
    env.objc.borrow::<UISegmentHostObject>(this).title
}
- (())setImage:(id)image {
    retain(env, image);
    let (old_image, image_view) = {
        let host = env.objc.borrow_mut::<UISegmentHostObject>(this);
        (std::mem::replace(&mut host.image, image), host.image_view)
    };
    release(env, old_image);
    () = msg![env; image_view setImage:image];
    layout_segment(env, this);
}
- (id)image {
    env.objc.borrow::<UISegmentHostObject>(this).image
}
- (())setTintColor:(id)tint_color {
    retain(env, tint_color);
    let old_tint = std::mem::replace(
        &mut env
            .objc
            .borrow_mut::<UISegmentHostObject>(this)
            .tint_color,
        tint_color,
    );
    release(env, old_tint);
    update_segment(env, this);
}

- (())setSelected:(bool)selected {
    () = msg_super![env; this setSelected:selected];
    update_segment(env, this);
}
- (())setEnabled:(bool)enabled {
    () = msg_super![env; this setEnabled:enabled];
    update_segment(env, this);
}
- (())layoutSubviews {
    layout_segment(env, this);
    msg_super![env; this layoutSubviews]
}

@end

};
