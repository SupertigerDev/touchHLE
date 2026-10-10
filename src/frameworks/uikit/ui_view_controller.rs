/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIViewController`.
//!
//! Resources:
//! - [View Controller Programming Guide for iOS (Legacy)](https://developer.apple.com/library/archive/documentation/WindowsViews/Conceptual/ViewControllerPGforiOSLegacy/BasicViewControllers/BasicViewControllers.html)

use crate::frameworks::core_graphics::CGRect;
use crate::frameworks::foundation::ns_objc_runtime::NSStringFromClass;
use crate::frameworks::foundation::ns_string::{from_rust_string, get_static_str, to_rust_string};
use crate::frameworks::foundation::NSInteger;
use crate::frameworks::uikit::ui_application::{
    UIInterfaceOrientation, UIInterfaceOrientationPortrait,
};
use crate::frameworks::uikit::ui_view::set_view_controller;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, todo_objc_setter, Class, ClassExports,
    HostObject, NSZonePtr,
};
use crate::Environment;

pub mod ui_navigation_controller;
pub mod ui_tab_bar_controller;

#[derive(Default)]
struct UIViewControllerHostObject {
    /// The root view.
    /// `UIView*`
    view: id,
    /// Nib name to be used at the load
    /// of the root view, may be nil.
    /// `NSString*`
    nib_name: id,
    /// Bundle to be used for load
    /// of the nib by name, may be nil.
    /// `NSBundle*`
    bundle: id,
    /// The title displayed for this controller.
    title: id,
    /// The tab bar item associated with this controller.
    tab_bar_item: id,
    /// The tab bar controller containing this controller, if any.
    tab_bar_controller: id,
    /// The navigation controller containing this controller, if any.
    navigation_controller: id,
    /// The view controller presented by this controller, if any.
    presented_view_controller: id,
    /// The view controller presenting this controller, if any.
    presenting_view_controller: id,
}
impl HostObject for UIViewControllerHostObject {}

type UIModalTransitionStyle = NSInteger;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIViewController: UIResponder

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIViewControllerHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// TODO: this should be a designated initializer
- (id)initWithNibName:(id)nib_name // NSString *
               bundle:(id)bundle { // NSBundle *
    retain(env, nib_name);
    retain(env, bundle);

    log_dbg!("[(UIViewController*){:?} initWithNibName:{:?} bundle:{:?}]", this, nib_name, bundle);

    env.objc.borrow_mut::<UIViewControllerHostObject>(this).nib_name = nib_name;
    env.objc.borrow_mut::<UIViewControllerHostObject>(this).bundle = bundle;

    this
}

- (id)initWithCoder:(id)coder {
    let key_ns_string = get_static_str(env, "UIView");
    let view: id = msg![env; coder decodeObjectForKey:key_ns_string];

    () = msg![env; this setView:view];

    let nib_name_key = get_static_str(env, "UINibName");
    let nib_name: id = msg![env; coder decodeObjectForKey:nib_name_key];
    retain(env, nib_name);
    env.objc.borrow_mut::<UIViewControllerHostObject>(this).nib_name = nib_name;

    let title_key = get_static_str(env, "UITitle");
    let title: id = msg![env; coder decodeObjectForKey:title_key];
    if title != nil {
        () = msg![env; this setTitle:title];
    }

    let tab_bar_item_key = get_static_str(env, "UITabBarItem");
    let tab_bar_item: id = msg![env; coder decodeObjectForKey:tab_bar_item_key];
    if tab_bar_item != nil {
        () = msg![env; this setTabBarItem:tab_bar_item];
    }

    this
}

- (())dealloc {
    let &UIViewControllerHostObject {
        view,
        nib_name,
        bundle,
        title,
        tab_bar_item,
        presented_view_controller,
        ..
    } = env.objc.borrow(this);

    if view != nil {
        set_view_controller(env, view, nil);
    }
    release(env, view);
    release(env, nib_name);
    release(env, bundle);
    release(env, title);
    release(env, tab_bar_item);
    release(env, presented_view_controller);

    env.objc.dealloc_object(this, &mut env.mem);
}

- (())loadView {
    let bundle: id = env.objc.borrow::<UIViewControllerHostObject>(this).bundle;
    let bundle: id = if bundle == nil {
        msg_class![env; NSBundle mainBundle]
    } else {
        bundle
    };

    let nib_name: id = get_nib_name(env, this, bundle);
    if nib_name != nil {
        // If we do have nib name, try to load it!
        log_dbg!(
            "Load {:?} view controller's view by nib, using name {}", this, to_rust_string(env, nib_name)
        );

        let nib: id = msg_class![env; UINib nibWithNibName:nib_name bundle:bundle];
        release(env, nib_name);

        // The NIB's File's Owner will be substituted by `this`,
        // implicitly loading the view as well
        let _: id = msg![env; nib instantiateWithOwner:this options:nil];

        let view = env.objc.borrow::<UIViewControllerHostObject>(this).view;
        // Having nil view at this point probably mean that
        // out nib's parsing is wrong.
        // Also we assume here the case of a "detached nib file"
        // TODO: support "integrated nib file"
        assert!(view != nil);

        return;
    };

    // As a last resort, use plain UIVIew for the root view
    let class: Class = msg![env; this class];
    log!("Unable to load {:?} {} view controller's view by nib, using plain UIView", this, env.objc.get_class_name(class).to_string());
    let view: id = msg_class![env; UIView alloc];
    // Docs are saying that "an empty UIView" is created,
    // but testing reveals that frame matches the screen one
    // (at least on the simulator)
    let screen: id = msg_class![env; UIScreen mainScreen];
    let app_frame: CGRect = msg![env; screen applicationFrame];
    let view: id = msg![env; view initWithFrame:app_frame];
    () = msg![env; this setView:view];
}

- (())setView:(id)new_view { // UIView*
    let host_obj = env.objc.borrow_mut::<UIViewControllerHostObject>(this);
    let old_view = std::mem::replace(&mut host_obj.view, new_view);
    if old_view != nil {
        set_view_controller(env, old_view, nil);
    }
    if new_view != nil {
        set_view_controller(env, new_view, this);
    }
    retain(env, new_view);
    release(env, old_view);
}
- (id)view {
    let view = env.objc.borrow_mut::<UIViewControllerHostObject>(this).view;
    if view == nil {
        () = msg![env; this loadView];
        let view = env.objc.borrow_mut::<UIViewControllerHostObject>(this).view;
        () = msg![env; this viewDidLoad];
        view
    } else {
        view
    }
}

// Usually overridden by the application
- (())viewDidLoad {
    log_dbg!("[(UIViewController*){:?} viewDidLoad]", this);
}
- (())viewWillAppear:(bool)animated {
    log_dbg!("[(UIViewController*){:?} viewWillAppear:{}]", this, animated);
}
- (())viewDidAppear:(bool)animated {
    log_dbg!("[(UIViewController*){:?} viewDidAppear:{}]", this, animated);
}
- (())viewWillDisappear:(bool)animated {
    log_dbg!("[(UIViewController*){:?} viewWillDisappear:{}]", this, animated);
}
- (())viewDidDisappear:(bool)animated {
    log_dbg!("[(UIViewController*){:?} viewDidDisappear:{}]", this, animated);
}

- (())setTitle:(id)title { // NSString *
    retain(env, title);
    let old_title = std::mem::replace(
        &mut env.objc.borrow_mut::<UIViewControllerHostObject>(this).title,
        title,
    );
    let tab_bar_item = env
        .objc
        .borrow::<UIViewControllerHostObject>(this)
        .tab_bar_item;
    if tab_bar_item != nil {
        let item_title: id = msg![env; tab_bar_item title];
        if item_title == old_title {
            () = msg![env; tab_bar_item setTitle:title];
        }
    }
    release(env, old_title);
}
- (id)title {
    env.objc.borrow::<UIViewControllerHostObject>(this).title
}
- (id)tabBarItem {
    env.objc.borrow::<UIViewControllerHostObject>(this).tab_bar_item
}
- (())setTabBarItem:(id)item {
    retain(env, item);
    let old_item = std::mem::replace(
        &mut env.objc.borrow_mut::<UIViewControllerHostObject>(this).tab_bar_item,
        item,
    );
    release(env, old_item);
    let tab_bar_controller = env
        .objc
        .borrow::<UIViewControllerHostObject>(this)
        .tab_bar_controller;
    if tab_bar_controller != nil {
        () = msg![env; tab_bar_controller _touchHLE_refreshTabBar];
    }
}
- (id)tabBarController {
    env.objc
        .borrow::<UIViewControllerHostObject>(this)
        .tab_bar_controller
}
- (id)navigationController {
    env.objc
        .borrow::<UIViewControllerHostObject>(this)
        .navigation_controller
}
- (())setEditing:(bool)editing {
    todo_objc_setter!(this, editing);
}
- (())setWantsFullScreenLayout:(bool)wants {
    todo_objc_setter!(this, wants);
}
- (())setHidesBottomBarWhenPushed:(bool)hides {
    todo_objc_setter!(this, hides);
}
- (())setSearchDisplayController:(id)controller { // UISearchDisplayController*
    todo_objc_setter!(this, controller);
}
- (id)searchDisplayController {
    nil
}
- (())setModalTransitionStyle:(UIModalTransitionStyle)style {
    todo_objc_setter!(this, style);
}

- (())presentViewController:(id)view_controller
                    animated:(bool)animated
                  completion:(id)_completion {
    if view_controller == nil {
        panic!("Cannot present a nil view controller");
    }

    let existing = env
        .objc
        .borrow::<UIViewControllerHostObject>(this)
        .presented_view_controller;
    if existing != nil {
        panic!("UIViewController already has a presented view controller");
    }

    let application: id = msg_class![env; UIApplication sharedApplication];
    let window: id = msg![env; application keyWindow];
    if window == nil {
        panic!("Cannot present a view controller without a key window");
    }

    let view: id = msg![env; view_controller view];
    let frame: CGRect = msg![env; window bounds];
    () = msg![env; view setFrame:frame];
    () = msg![env; view_controller viewWillAppear:animated];
    () = msg![env; window addSubview:view];
    () = msg![env; view_controller viewDidAppear:animated];

    retain(env, view_controller);
    env.objc
        .borrow_mut::<UIViewControllerHostObject>(this)
        .presented_view_controller = view_controller;
    env.objc
        .borrow_mut::<UIViewControllerHostObject>(view_controller)
        .presenting_view_controller = this;
}

- (())dismissViewControllerAnimated:(bool)animated
                               completion:(id)_completion {
    let presented = env
        .objc
        .borrow::<UIViewControllerHostObject>(this)
        .presented_view_controller;
    let presenter = if presented == nil {
        env.objc
            .borrow::<UIViewControllerHostObject>(this)
            .presenting_view_controller
    } else {
        this
    };
    let dismissed = if presented == nil { this } else { presented };

    if presenter == nil {
        return;
    }

    let view: id = msg![env; dismissed view];
    () = msg![env; dismissed viewWillDisappear:animated];
    () = msg![env; view removeFromSuperview];
    () = msg![env; dismissed viewDidDisappear:animated];

    env.objc
        .borrow_mut::<UIViewControllerHostObject>(presenter)
        .presented_view_controller = nil;
    env.objc
        .borrow_mut::<UIViewControllerHostObject>(dismissed)
        .presenting_view_controller = nil;
    release(env, dismissed);
}

- (())dismissModalViewControllerAnimated:(bool)animated {
    () = msg![env; this dismissViewControllerAnimated:animated completion:nil];
}
- (())presentModalViewController:(id)view_controller
                         animated:(bool)animated {
    () = msg![env; this presentViewController:view_controller animated:animated completion:nil];
}
- (())dismissMoviePlayerViewControllerAnimated {
    log!("TODO: [(UIViewController*){:?} dismissMoviePlayerViewControllerAnimated]", this); // TODO
}

- (bool)shouldAutorotateToInterfaceOrientation:(UIInterfaceOrientation)interface_orientation {
    interface_orientation == UIInterfaceOrientationPortrait
}

// UIResponder implementation
// From the Apple UIView docs regarding [UIResponder nextResponder]:
// "UIViewController similarly implements the method
// and returns its view’s superview."
// https://developer.apple.com/documentation/uikit/uiresponder/next?language=objc
- (id)nextResponder {
    let view = msg![env; this view];
    let next_responder = msg![env; view superview];
    log_dbg!("[(UIView*){:?} nextResponder] => {:?}", this, next_responder);
    next_responder
}

@end

};

/// A helper function to resolve suitable NIB name for a `view_controller`
/// in the `bundle`. Returns nil if fails.
///
/// Note: It's a responsibility of a caller to release the returned name
/// if not-nil!
fn get_nib_name(env: &mut Environment, view_controller: id, bundle: id) -> id {
    let provider_nib_name: id = env
        .objc
        .borrow::<UIViewControllerHostObject>(view_controller)
        .nib_name;
    if provider_nib_name != nil {
        // TODO: it's not clear how to handle situation when
        // provided nib name do not exist in the bundle.
        // It probably means that our bundle resource loading
        // is faulty, to check
        assert!(check_nib_exists(env, bundle, provider_nib_name));

        retain(env, provider_nib_name);
        return provider_nib_name;
    };

    let class: Class = msg![env; view_controller class];
    let class_name: id = NSStringFromClass(env, class);
    let class_name_str = to_rust_string(env, class_name);

    if let Some(name) = class_name_str.strip_suffix("Controller") {
        let ns_name: id = from_rust_string(env, name.to_string());
        if check_nib_exists(env, bundle, ns_name) {
            release(env, class_name);
            return ns_name;
        }
    }

    if check_nib_exists(env, bundle, class_name) {
        class_name
    } else {
        release(env, class_name);
        nil
    }
}

/// A helper function to check if `nib_name` NIB actually
/// existing in the `bundle`
fn check_nib_exists(env: &mut Environment, bundle: id, nib_name: id) -> bool {
    let type_: id = get_static_str(env, "nib");
    let res: id = msg![env; bundle pathForResource:nib_name ofType:type_];
    res != nil
}
