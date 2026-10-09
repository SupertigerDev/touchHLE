/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIAlertView`.

use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string;
use crate::frameworks::uikit::ui_font::UITextAlignmentCenter;
use crate::frameworks::uikit::ui_view::ui_control::UIControlEventTouchUpInside;
use crate::frameworks::uikit::ui_view::UIViewHostObject;
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes,
    release, retain, ClassExports, NSZonePtr,
};

const UIControlStateNormal: u32 = 0;

struct UIAlertViewHostObject {
    superclass: UIViewHostObject,
    title: id,
    message: id,
    delegate: id,
    cancel_button_title: id,
}
impl_HostObject_with_superclass!(UIAlertViewHostObject);
impl Default for UIAlertViewHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            title: nil,
            message: nil,
            delegate: nil,
            cancel_button_title: nil,
        }
    }
}

fn make_label(env: &mut crate::Environment, text: id, frame: CGRect, lines: i32) -> id {
    let label: id = msg_class![env; UILabel alloc];
    let label: id = msg![env; label initWithFrame:frame];
    () = msg![env; label setText:text];
    () = msg![env; label setTextAlignment:UITextAlignmentCenter];
    () = msg![env; label setNumberOfLines:lines];
    autorelease(env, label)
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIAlertView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIAlertViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithTitle:(id)title
                      message:(id)message
                     delegate:(id)delegate
            cancelButtonTitle:(id)cancelButtonTitle
            otherButtonTitles:(id)otherButtonTitles {
    let this = msg_super![env; this init];
    let host = env.objc.borrow_mut::<UIAlertViewHostObject>(this);
    host.title = title;
    host.message = message;
    host.delegate = delegate;
    host.cancel_button_title = cancelButtonTitle;
    retain(env, title);
    retain(env, message);
    retain(env, delegate);
    retain(env, cancelButtonTitle);
    if otherButtonTitles != nil {
        log!("TODO: UIAlertView additional button titles are not implemented");
    }
    this
}

- (())dealloc {
    let &UIAlertViewHostObject {
        superclass: _,
        title,
        message,
        delegate,
        cancel_button_title,
    } = env.objc.borrow(this);
    release(env, title);
    release(env, message);
    release(env, delegate);
    release(env, cancel_button_title);
    msg_super![env; this dealloc]
}

- (())addButtonWithTitle:(id)title {
    log!("TODO: UIAlertView addButtonWithTitle:{}",
        ns_string::to_rust_string(env, title));
}

- (())setDelegate:(id)delegate {
    retain(env, delegate);
    let old_delegate = std::mem::replace(
        &mut env.objc.borrow_mut::<UIAlertViewHostObject>(this).delegate,
        delegate,
    );
    release(env, old_delegate);
}

- (())show {
    let (title, message, cancel_button_title) = {
        let host = env.objc.borrow::<UIAlertViewHostObject>(this);
        (host.title, host.message, host.cancel_button_title)
    };
    let application: id = msg_class![env; UIApplication sharedApplication];
    let window: id = msg![env; application keyWindow];
    if window == nil {
        log!("UIAlertView show skipped because there is no key window");
        return;
    }

    let bounds: CGRect = msg![env; window bounds];
    () = msg![env; this setFrame:bounds];
    let dim_color_value: f32 = 0.0;
    let dim_alpha: f32 = 0.45;
    let dim_color: id = msg_class![env; UIColor colorWithWhite:dim_color_value alpha:dim_alpha];
    () = msg![env; this setBackgroundColor:dim_color];

    let panel_width = (bounds.size.width - 40.0).min(320.0).max(200.0);
    let panel_height = 156.0;
    let panel_frame = CGRect {
        origin: CGPoint {
            x: (bounds.size.width - panel_width) / 2.0,
            y: (bounds.size.height - panel_height) / 2.0,
        },
        size: CGSize {
            width: panel_width,
            height: panel_height,
        },
    };
    let panel: id = msg_class![env; UIView alloc];
    let panel: id = msg![env; panel initWithFrame:panel_frame];
    let panel_color: id = msg_class![env; UIColor whiteColor];
    () = msg![env; panel setBackgroundColor:panel_color];

    let title_text = if title == nil {
        ns_string::from_rust_string(env, String::new())
    } else {
        title
    };
    let message_text = if message == nil {
        ns_string::from_rust_string(env, String::new())
    } else {
        message
    };
    let title_frame = CGRect {
        origin: CGPoint { x: 12.0, y: 12.0 },
        size: CGSize {
            width: panel_width - 24.0,
            height: 28.0,
        },
    };
    let title_label = make_label(env, title_text, title_frame, 1);
    () = msg![env; panel addSubview:title_label];
    if title == nil {
        release(env, title_text);
    }

    let message_frame = CGRect {
        origin: CGPoint { x: 16.0, y: 44.0 },
        size: CGSize {
            width: panel_width - 32.0,
            height: 54.0,
        },
    };
    let message_label = make_label(env, message_text, message_frame, 0);
    () = msg![env; panel addSubview:message_label];
    if message == nil {
        release(env, message_text);
    }

    let button_frame = CGRect {
        origin: CGPoint {
            x: 16.0,
            y: panel_height - 50.0,
        },
        size: CGSize {
            width: panel_width - 32.0,
            height: 38.0,
        },
    };
    let button_title = if cancel_button_title == nil {
        ns_string::from_rust_string(env, "OK".to_owned())
    } else {
        cancel_button_title
    };
    let button: id = msg_class![env; UIButton buttonWithType:1];
    () = msg![env; button setFrame:button_frame];
    () = msg![env; button setTitle:button_title forState:UIControlStateNormal];
    let dismiss_selector = env.objc.lookup_selector("_touchHLE_dismiss:").unwrap();
    () = msg![env; button addTarget:this action:dismiss_selector
                  forControlEvents:UIControlEventTouchUpInside];
    () = msg![env; panel addSubview:button];
    if cancel_button_title == nil {
        release(env, button_title);
    }

    () = msg![env; this addSubview:panel];
    release(env, panel);
    () = msg![env; window addSubview:this];
    log!("UIAlertView shown: title {:?}, message {:?}",
        if title == nil { "(nil)".into() } else { ns_string::to_rust_string(env, title) },
        if message == nil { "(nil)".into() } else { ns_string::to_rust_string(env, message) });
}

- (())_touchHLE_dismiss:(id)_sender {
    let delegate = env.objc.borrow::<UIAlertViewHostObject>(this).delegate;
    if delegate != nil {
        let selector = env
            .objc
            .lookup_selector("alertView:clickedButtonAtIndex:")
            .unwrap();
        let responds: bool = msg![env; delegate respondsToSelector:selector];
        if responds {
            () = msg![env; delegate alertView:this clickedButtonAtIndex:0i32];
        }
    }
    () = msg![env; this removeFromSuperview];
}

@end

};
