/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! UIKit gesture recognizers.

use crate::frameworks::core_graphics::CGPoint;
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::objc::{id, msg, msg_send, nil, objc_classes, ClassExports, HostObject, NSZonePtr, SEL};
use crate::Environment;

const UIGESTURE_RECOGNIZER_STATE_POSSIBLE: NSInteger = 0;
const UIGESTURE_RECOGNIZER_STATE_ENDED: NSInteger = 3;
const UISWIPE_GESTURE_RECOGNIZER_DIRECTION_RIGHT: NSUInteger = 1;
const UISWIPE_GESTURE_RECOGNIZER_DIRECTION_LEFT: NSUInteger = 2;
const UISWIPE_GESTURE_RECOGNIZER_DIRECTION_UP: NSUInteger = 4;
const UISWIPE_GESTURE_RECOGNIZER_DIRECTION_DOWN: NSUInteger = 8;
const MULTI_TAP_INTERVAL: f64 = 0.35;
const MULTI_TAP_DISTANCE_SQUARED: f32 = 100.0;
const SWIPE_MINIMUM_DISTANCE: f32 = 30.0;

struct GestureRecognizerHostObject {
    target: id,
    action: Option<SEL>,
    view: id,
    delegate: id,
    required_to_fail: Vec<id>,
    enabled: bool,
    cancels_touches_in_view: bool,
    delays_touches_began: bool,
    delays_touches_ended: bool,
    number_of_taps_required: NSUInteger,
    number_of_touches_required: NSUInteger,
    is_swipe: bool,
    swipe_direction: NSUInteger,
    state: NSInteger,
    location: CGPoint,
    start_location: Option<CGPoint>,
    tap_count: NSUInteger,
    last_tap_timestamp: f64,
    last_tap_location: CGPoint,
}
impl HostObject for GestureRecognizerHostObject {}

impl Default for GestureRecognizerHostObject {
    fn default() -> Self {
        Self {
            target: nil,
            action: None,
            view: nil,
            delegate: nil,
            required_to_fail: Vec::new(),
            enabled: true,
            cancels_touches_in_view: true,
            delays_touches_began: true,
            delays_touches_ended: true,
            number_of_taps_required: 1,
            number_of_touches_required: 1,
            is_swipe: false,
            swipe_direction: UISWIPE_GESTURE_RECOGNIZER_DIRECTION_RIGHT,
            state: UIGESTURE_RECOGNIZER_STATE_POSSIBLE,
            location: CGPoint::default(),
            start_location: None,
            tap_count: 0,
            last_tap_timestamp: 0.0,
            last_tap_location: CGPoint::default(),
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIGestureRecognizer: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc
        .alloc_object(this, Box::<GestureRecognizerHostObject>::default(), &mut env.mem)
}

- (id)initWithTarget:(id)target action:(SEL)action {
    let class: crate::objc::Class = msg![env; this class];
    let is_swipe = env.objc.get_class_name(class) == "UISwipeGestureRecognizer";
    let host_object = env.objc.borrow_mut::<GestureRecognizerHostObject>(this);
    host_object.target = target;
    host_object.action = (!action.is_null()).then_some(action);
    host_object.is_swipe = is_swipe;
    this
}

- (())addTarget:(id)target action:(SEL)action {
    let host_object = env.objc.borrow_mut::<GestureRecognizerHostObject>(this);
    host_object.target = target;
    host_object.action = (!action.is_null()).then_some(action);
}

- (())addTarget:(id)target action:(SEL)action forControlEvents:(NSUInteger)_control_events {
    () = msg![env; this addTarget:target action:action];
}

- (id)initWithCoder:(id)coder {
    let class: crate::objc::Class = msg![env; this class];
    let is_swipe = env.objc.get_class_name(class) == "UISwipeGestureRecognizer";

    let enabled_key = get_static_str(env, "UIGestureRecognizerEnabled");
    let has_enabled: bool = msg![env; coder containsValueForKey:enabled_key];
    let enabled = if has_enabled {
        Some(msg![env; coder decodeBoolForKey:enabled_key])
    } else {
        None
    };
    let touches_key = get_static_str(env, "UIGestureRecognizerNumberOfTouchesRequired");
    let has_touches: bool = msg![env; coder containsValueForKey:touches_key];
    let touches: Option<NSInteger> = if has_touches {
        Some(msg![env; coder decodeIntegerForKey:touches_key])
    } else {
        None
    };
    let taps_key = get_static_str(env, "UITapGestureRecognizerNumberOfTapsRequired");
    let has_taps: bool = msg![env; coder containsValueForKey:taps_key];
    let taps: Option<NSInteger> = if has_taps {
        Some(msg![env; coder decodeIntegerForKey:taps_key])
    } else {
        None
    };
    let direction_key = get_static_str(env, "UISwipeGestureRecognizer.direction");
    let has_direction: bool = msg![env; coder containsValueForKey:direction_key];
    let direction: Option<NSInteger> = if is_swipe && has_direction {
        Some(msg![env; coder decodeIntegerForKey:direction_key])
    } else {
        None
    };

    let host_object = env.objc.borrow_mut::<GestureRecognizerHostObject>(this);
    host_object.is_swipe = is_swipe;
    if let Some(enabled) = enabled {
        host_object.enabled = enabled;
    }
    if let Some(touches) = touches {
        host_object.number_of_touches_required = touches.try_into().unwrap();
    }
    if let Some(taps) = taps {
        host_object.number_of_taps_required = taps.try_into().unwrap();
    }
    if let Some(direction) = direction {
        host_object.swipe_direction = direction.try_into().unwrap();
    }
    this
}

- (())dealloc {
    let GestureRecognizerHostObject { view: _, .. } =
        std::mem::take(env.objc.borrow_mut(this));
    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)view {
    env.objc.borrow::<GestureRecognizerHostObject>(this).view
}

- (id)delegate {
    env.objc.borrow::<GestureRecognizerHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    env.objc
        .borrow_mut::<GestureRecognizerHostObject>(this)
        .delegate = delegate;
}

- (())requireGestureRecognizerToFail:(id)other {
    if other == nil || other == this {
        return;
    }
    let required_to_fail = &mut env
        .objc
        .borrow_mut::<GestureRecognizerHostObject>(this)
        .required_to_fail;
    if !required_to_fail.contains(&other) {
        required_to_fail.push(other);
    }
}

- (bool)canPreventGestureRecognizer:(id)_other {
    true
}

- (bool)canBePreventedByGestureRecognizer:(id)_other {
    true
}

- (bool)isEnabled {
    env.objc.borrow::<GestureRecognizerHostObject>(this).enabled
}
- (())setEnabled:(bool)enabled {
    let host_object = env.objc.borrow_mut::<GestureRecognizerHostObject>(this);
    host_object.enabled = enabled;
    host_object.state = UIGESTURE_RECOGNIZER_STATE_POSSIBLE;
    host_object.tap_count = 0;
}

- (NSInteger)state {
    env.objc.borrow::<GestureRecognizerHostObject>(this).state
}

- (NSUInteger)numberOfTapsRequired {
    env.objc.borrow::<GestureRecognizerHostObject>(this).number_of_taps_required
}
- (())setNumberOfTapsRequired:(NSUInteger)count {
    assert!(count > 0);
    env.objc
        .borrow_mut::<GestureRecognizerHostObject>(this)
        .number_of_taps_required = count;
}

- (NSUInteger)numberOfTouchesRequired {
    env.objc.borrow::<GestureRecognizerHostObject>(this).number_of_touches_required
}
- (())setNumberOfTouchesRequired:(NSUInteger)count {
    assert!(count > 0);
    env.objc
        .borrow_mut::<GestureRecognizerHostObject>(this)
        .number_of_touches_required = count;
}

- (NSUInteger)direction {
    env.objc
        .borrow::<GestureRecognizerHostObject>(this)
        .swipe_direction
}
- (())setDirection:(NSUInteger)direction {
    assert_ne!(direction, 0);
    assert_eq!(direction & !0x0f, 0);
    env.objc
        .borrow_mut::<GestureRecognizerHostObject>(this)
        .swipe_direction = direction;
}

- (bool)cancelsTouchesInView {
    env.objc
        .borrow::<GestureRecognizerHostObject>(this)
        .cancels_touches_in_view
}
- (())setCancelsTouchesInView:(bool)cancels {
    env.objc
        .borrow_mut::<GestureRecognizerHostObject>(this)
        .cancels_touches_in_view = cancels;
}

- (bool)delaysTouchesBegan {
    env.objc
        .borrow::<GestureRecognizerHostObject>(this)
        .delays_touches_began
}
- (())setDelaysTouchesBegan:(bool)delays {
    env.objc
        .borrow_mut::<GestureRecognizerHostObject>(this)
        .delays_touches_began = delays;
}

- (bool)delaysTouchesEnded {
    env.objc
        .borrow::<GestureRecognizerHostObject>(this)
        .delays_touches_ended
}
- (())setDelaysTouchesEnded:(bool)delays {
    env.objc
        .borrow_mut::<GestureRecognizerHostObject>(this)
        .delays_touches_ended = delays;
}

- (CGPoint)locationInView:(id)view {
    let location = env.objc.borrow::<GestureRecognizerHostObject>(this).location;
    let recognizer_view = env.objc.borrow::<GestureRecognizerHostObject>(this).view;
    if view == nil || view == recognizer_view {
        location
    } else {
        msg![env; view convertPoint:location fromView:recognizer_view]
    }
}

@end

@implementation UITapGestureRecognizer: UIGestureRecognizer
@end

@implementation UISwipeGestureRecognizer: UIGestureRecognizer
@end

};

pub(super) fn set_view(env: &mut Environment, recognizer: id, view: id) {
    env.objc
        .borrow_mut::<GestureRecognizerHostObject>(recognizer)
        .view = view;
}

pub(super) fn handle_touches_ended(env: &mut Environment, view: id, touches: id) {
    let recognizers = env
        .objc
        .borrow::<super::ui_view::UIViewHostObject>(view)
        .gesture_recognizers
        .clone();

    let touch_array: id = msg![env; touches allObjects];
    let touch_count: NSUInteger = msg![env; touch_array count];
    if touch_count == 0 {
        return;
    }
    for recognizer in recognizers {
        let (enabled, required_touches, action, is_swipe, required_taps, target, start_location) = {
            let snapshot = env.objc.borrow::<GestureRecognizerHostObject>(recognizer);
            (
                snapshot.enabled,
                snapshot.number_of_touches_required,
                snapshot.action,
                snapshot.is_swipe,
                snapshot.number_of_taps_required,
                snapshot.target,
                snapshot.start_location,
            )
        };
        let Some(action) = action else {
            continue;
        };
        if !enabled || required_touches != touch_count {
            continue;
        }

        let first_touch: id = msg![env; touch_array objectAtIndex:0u32];
        let location: CGPoint = msg![env; first_touch locationInView:view];
        let timestamp: f64 = msg![env; first_touch timestamp];

        let should_fire = if is_swipe {
            let Some(start_location) = start_location else {
                continue;
            };
            let dx = location.x - start_location.x;
            let dy = location.y - start_location.y;
            let direction = if dx.abs() >= dy.abs() {
                if dx >= SWIPE_MINIMUM_DISTANCE {
                    UISWIPE_GESTURE_RECOGNIZER_DIRECTION_RIGHT
                } else if dx <= -SWIPE_MINIMUM_DISTANCE {
                    UISWIPE_GESTURE_RECOGNIZER_DIRECTION_LEFT
                } else {
                    0
                }
            } else if dy >= SWIPE_MINIMUM_DISTANCE {
                UISWIPE_GESTURE_RECOGNIZER_DIRECTION_DOWN
            } else if dy <= -SWIPE_MINIMUM_DISTANCE {
                UISWIPE_GESTURE_RECOGNIZER_DIRECTION_UP
            } else {
                0
            };
            let state = env
                .objc
                .borrow_mut::<GestureRecognizerHostObject>(recognizer);
            state.start_location = None;
            state.location = location;
            if direction != 0 && state.swipe_direction & direction != 0 {
                state.state = UIGESTURE_RECOGNIZER_STATE_ENDED;
                true
            } else {
                false
            }
        } else {
            let state = env
                .objc
                .borrow_mut::<GestureRecognizerHostObject>(recognizer);
            let elapsed = timestamp - state.last_tap_timestamp;
            let dx = location.x - state.last_tap_location.x;
            let dy = location.y - state.last_tap_location.y;
            if state.tap_count > 0
                && elapsed <= MULTI_TAP_INTERVAL
                && dx * dx + dy * dy <= MULTI_TAP_DISTANCE_SQUARED
            {
                state.tap_count += 1;
            } else {
                state.tap_count = 1;
            }
            state.last_tap_timestamp = timestamp;
            state.last_tap_location = location;
            state.location = location;
            if state.tap_count >= required_taps {
                state.tap_count = 0;
                state.state = UIGESTURE_RECOGNIZER_STATE_ENDED;
                true
            } else {
                false
            }
        };

        if should_fire {
            log_dbg!(
                "Recognized {} gesture with recognizer {:?} on view {:?}",
                if is_swipe { "swipe" } else { "tap" },
                recognizer,
                view,
            );
            () = msg_send(env, (target, action, recognizer));
            env.objc
                .borrow_mut::<GestureRecognizerHostObject>(recognizer)
                .state = UIGESTURE_RECOGNIZER_STATE_POSSIBLE;
        }
    }
}

pub(super) fn handle_touches_began(env: &mut Environment, view: id, touches: id) {
    let recognizers = env
        .objc
        .borrow::<super::ui_view::UIViewHostObject>(view)
        .gesture_recognizers
        .clone();
    let touch_array: id = msg![env; touches allObjects];
    let touch_count: NSUInteger = msg![env; touch_array count];
    if touch_count == 0 {
        return;
    }
    let first_touch: id = msg![env; touch_array objectAtIndex:0u32];
    let location: CGPoint = msg![env; first_touch locationInView:view];

    for recognizer in recognizers {
        let state = env
            .objc
            .borrow_mut::<GestureRecognizerHostObject>(recognizer);
        if state.enabled && state.number_of_touches_required == touch_count {
            state.start_location = Some(location);
            state.location = location;
        }
    }
}
