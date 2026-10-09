/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UINavigationController`, `UINavigationBar` and `UINavigationItem`.

use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{ns_array, NSInteger, NSUInteger};
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_super, nil, objc_classes, release,
    retain, ClassExports, HostObject, NSZonePtr, SEL,
};
use crate::todo_objc_setter;

// TODO: navigation bar and toolbar
// TODO: animations

#[derive(Default)]
struct UINavigationControllerHostObject {
    superclass: super::UIViewControllerHostObject,
    /// something implementing UINavigationControllerDelegate
    delegate: id,
    /// Navigation stack of view controllers, non-retaining
    /// (we explicitly retain/release on push/pop messages)
    navigation_stack: Vec<id>,
}
impl_HostObject_with_superclass!(UINavigationControllerHostObject);

#[derive(Default)]
struct UINavigationItemHostObject {
    title: id,
}
impl HostObject for UINavigationItemHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UINavigationController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UINavigationControllerHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    let key = get_static_str(env, "UIViewControllers");
    let view_controllers: id = msg![env; coder decodeObjectForKey:key];
    () = msg![env; this setViewControllers:view_controllers];
    this
}

- (id)initWithRootViewController:(id)root_vc { // UIViewController *
    () = msg![env; this pushViewController:root_vc animated:false];
    this
}

// weak/non-retaining
- (())setDelegate:(id)delegate { // something implementing UINavigationControllerDelegate
    log_dbg!("[(UINavigationController*){:?} setDelegate:{:?}]", this, delegate);
    let host_object = env.objc.borrow_mut::<UINavigationControllerHostObject>(this);
    host_object.delegate = delegate;
}
- (id)delegate {
    env.objc.borrow::<UINavigationControllerHostObject>(this).delegate
}

- (())pushViewController:(id)view_controller // UIViewController *
                animated:(bool)_animated {
    let stack = &mut env.objc.borrow_mut::<UINavigationControllerHostObject>(this).navigation_stack;
    assert!(!stack.contains(&view_controller));
    stack.push(view_controller);
    retain(env, view_controller);
    env.objc
        .borrow_mut::<super::UIViewControllerHostObject>(view_controller)
        .navigation_controller = this;

    let delegate = env.objc.borrow::<UINavigationControllerHostObject>(this).delegate;
    let sel: SEL = env
        .objc
        .register_host_selector(
            "navigationController:willShowViewController:animated:".to_string(),
            &mut env.mem
        );
    let responds: bool = msg![env; delegate respondsToSelector:sel];
    if responds {
        () = msg![env; delegate navigationController:this willShowViewController:view_controller animated:false];
    }
    let self_view: id = msg![env; this view];
    let vc_view: id = msg![env; view_controller view];
    // TODO: animations
    () = msg![env; view_controller viewWillAppear:false];
    () = msg![env; self_view addSubview:vc_view];
    () = msg![env; view_controller viewDidAppear:false];
    let sel: SEL = env
        .objc
        .register_host_selector(
            "navigationController:didShowViewController:animated:".to_string(),
            &mut env.mem
        );
    let responds: bool  = msg![env; delegate respondsToSelector:sel];
    if responds {
        () = msg![env; delegate navigationController:this didShowViewController:view_controller animated:false];
    }
}

- (id)topViewController {
    if let Some(top_vc) = env.objc.borrow::<UINavigationControllerHostObject>(this).navigation_stack.last() {
        *top_vc
    } else {
        nil
    }
}

- (id)viewControllers {
    let vcs = env.objc.borrow::<UINavigationControllerHostObject>(this).navigation_stack.to_vec();
    for vc in &vcs {
        retain(env, *vc);
    }
    let res = ns_array::from_vec(env, vcs);
    autorelease(env, res)
}
- (())setViewControllers:(id)controllers { // NSArray *
    msg![env; this setViewControllers:controllers animated:false]
}

- (())setViewControllers:(id)controllers // NSArray *
                animated:(bool)animated {
    assert!(!animated);

    // Clean existing view controllers
    let self_view = env.objc.borrow::<UINavigationControllerHostObject>(this).superclass.view;
    let mut stack = std::mem::take(&mut env.objc.borrow_mut::<UINavigationControllerHostObject>(this).navigation_stack);
    // TODO: shall we drain in reverse order? does it matter?
    for controller in stack.drain(..) {
        let vc_view = env.objc.borrow::<super::UIViewControllerHostObject>(controller).view;
        let vc_view_superview = msg![env; vc_view superview];
        assert_eq!(self_view, vc_view_superview);
        // TODO: view{Will,Did}Disappear: messages for vc?
        () = msg![env; vc_view removeFromSuperview];

        release(env, controller);
    }

    let mut tmp_stack: Vec<id> = Vec::new();
    let count: NSUInteger = msg![env; controllers count];
    // TODO: zero count
    assert!(count > 0);
    for i in 0..(count - 1) {
        let next: id = msg![env; controllers objectAtIndex:i];
        tmp_stack.push(next);
        retain(env, next);
    }
    env.objc.borrow_mut::<UINavigationControllerHostObject>(this).navigation_stack = tmp_stack;

    // The n-1 element in the controllers array is special and need to be pushed
    // TODO: double check this behavior
    let last_vc: id = msg![env; controllers objectAtIndex:(count - 1)];
    () = msg![env; this pushViewController:last_vc animated:animated];
}

- (id)navigationBar {
    // TODO
    nil
}
- (())setNavigationBarHidden:(bool)_hidden {
    // TODO
}

@end

// TODO: actually draw the bar, its title and its buttons. For now this only
// exists so that nibs containing a navigation bar can be loaded.
@implementation UINavigationBar: UIView

- (())setBarStyle:(NSInteger)style {
    todo_objc_setter!(this, style);
}

- (())setTranslucent:(bool)translucent {
    todo_objc_setter!(this, translucent);
}

- (())setTintColor:(id)color { // UIColor *
    todo_objc_setter!(this, color);
}

- (())setItems:(id)items { // NSArray *
    todo_objc_setter!(this, items);
}

- (())setItems:(id)_items
      animated:(bool)_animated {
    log!("TODO: [(UINavigationBar*){:?} setItems:animated:]", this);
}

- (())pushNavigationItem:(id)_item // UINavigationItem *
                animated:(bool)_animated {
    log!("TODO: [(UINavigationBar*){:?} pushNavigationItem:animated:]", this);
}

- (id)popNavigationItemAnimated:(bool)_animated {
    log!("TODO: [(UINavigationBar*){:?} popNavigationItemAnimated:]", this);
    nil
}

@end

@implementation UINavigationItem: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UINavigationItemHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let title_key = get_static_str(env, "UITitle");
    let title: id = msg![env; coder decodeObjectForKey:title_key];
    msg![env; this initWithTitle:title]
}

- (id)initWithTitle:(id)title { // NSString *
    retain(env, title);
    env.objc.borrow_mut::<UINavigationItemHostObject>(this).title = title;
    this
}

- (())dealloc {
    let title = env.objc.borrow::<UINavigationItemHostObject>(this).title;
    release(env, title);
    msg_super![env; this dealloc]
}

- (id)title {
    env.objc.borrow::<UINavigationItemHostObject>(this).title
}
- (())setTitle:(id)title { // NSString *
    retain(env, title);
    let old_title = std::mem::replace(
        &mut env.objc.borrow_mut::<UINavigationItemHostObject>(this).title,
        title,
    );
    release(env, old_title);
}

- (())setTitleView:(id)view { // UIView *
    todo_objc_setter!(this, view);
}
- (())setLeftBarButtonItem:(id)item { // UIBarButtonItem *
    todo_objc_setter!(this, item);
}
- (())setRightBarButtonItem:(id)item { // UIBarButtonItem *
    todo_objc_setter!(this, item);
}
- (())setHidesBackButton:(bool)hides {
    todo_objc_setter!(this, hides);
}

@end

// TODO: actually display bar button items. For now this only lets apps
// create them without crashing.
@implementation UIBarButtonItem: NSObject

- (id)initWithCoder:(id)_coder {
    this
}
- (id)initWithBarButtonSystemItem:(NSInteger)_item
                           target:(id)_target
                           action:(SEL)_action {
    this
}
- (id)initWithTitle:(id)_title // NSString *
              style:(NSInteger)_style
             target:(id)_target
             action:(SEL)_action {
    this
}
- (id)initWithImage:(id)_image // UIImage *
              style:(NSInteger)_style
             target:(id)_target
             action:(SEL)_action {
    this
}
- (id)initWithCustomView:(id)_view { // UIView *
    this
}

- (())setTitle:(id)title { // NSString *
    todo_objc_setter!(this, title);
}
- (())setStyle:(NSInteger)style {
    todo_objc_setter!(this, style);
}
- (())setEnabled:(bool)enabled {
    todo_objc_setter!(this, enabled);
}
- (())setTarget:(id)target {
    todo_objc_setter!(this, target);
}
- (())setAction:(SEL)action {
    todo_objc_setter!(this, action);
}
- (())setTintColor:(id)color { // UIColor *
    todo_objc_setter!(this, color);
}

- (())addTarget:(id)_target
         action:(SEL)_action
forControlEvents:(NSUInteger)_events {
    log!("TODO: [(UIBarButtonItem*){:?} addTarget:action:forControlEvents:]", this);
}

@end

};
