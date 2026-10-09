/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UITabBarController`, `UITabBar`, and `UITabBarItem`.

use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{ns_array, NSInteger, NSUInteger};
use crate::frameworks::uikit::ui_view::ui_control::{
    UIControlEventTouchUpInside, UIControlStateNormal,
};
use crate::frameworks::uikit::ui_view_controller::UIViewControllerHostObject;
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes,
    release, retain, Class, ClassExports, HostObject, NSZonePtr, SEL,
};

const TAB_BAR_HEIGHT: f32 = 49.0;

#[derive(Default)]
struct UITabBarControllerHostObject {
    superclass: UIViewControllerHostObject,
    delegate: id,
    view_controllers: Vec<id>,
    selected_index: NSUInteger,
    tab_bar: id,
    displayed_controller: id,
    view_loaded: bool,
}
impl_HostObject_with_superclass!(UITabBarControllerHostObject);

#[derive(Default)]
struct UITabBarHostObject {
    superclass: super::super::ui_view::UIViewHostObject,
    delegate: id,
    items: Vec<id>,
    selected_item: id,
    buttons: Vec<id>,
}
impl_HostObject_with_superclass!(UITabBarHostObject);

#[derive(Default)]
struct UITabBarItemHostObject {
    title: id,
    image: id,
    tag: NSInteger,
}
impl HostObject for UITabBarItemHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITabBarController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UITabBarControllerHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    let key = get_static_str(env, "UIViewControllers");
    let view_controllers: id = msg![env; coder decodeObjectForKey:key];
    () = msg![env; this setViewControllers:view_controllers];
    this
}

- (())dealloc {
    let host_object = env.objc.borrow_mut::<UITabBarControllerHostObject>(this);
    let view_controllers = std::mem::take(&mut host_object.view_controllers);
    let tab_bar = std::mem::replace(&mut host_object.tab_bar, nil);
    for view_controller in view_controllers {
        env.objc.borrow_mut::<UIViewControllerHostObject>(view_controller).tab_bar_controller = nil;
        release(env, view_controller);
    }
    release(env, tab_bar);
    msg_super![env; this dealloc]
}

- (())viewDidLoad {
    let root_view: id = msg![env; this view];
    let frame: CGRect = msg![env; root_view bounds];
    let tab_bar_frame = CGRect {
        origin: CGPoint {
            x: frame.origin.x,
            y: frame.origin.y + frame.size.height - TAB_BAR_HEIGHT,
        },
        size: CGSize {
            width: frame.size.width,
            height: TAB_BAR_HEIGHT,
        },
    };
    let tab_bar_class: Class = msg_class![env; UITabBar class];
    let subviews: id = msg![env; root_view subviews];
    let subview_count: NSUInteger = msg![env; subviews count];
    let mut tab_bar: id = nil;
    for index in 0..subview_count {
        let subview: id = msg![env; subviews objectAtIndex:index];
        let is_tab_bar: bool = msg![env; subview isKindOfClass:tab_bar_class];
        if is_tab_bar {
            tab_bar = subview;
            break;
        }
    }
    let created_tab_bar = tab_bar == nil;
    if created_tab_bar {
        tab_bar = msg_class![env; UITabBar alloc];
        tab_bar = msg![env; tab_bar initWithFrame:tab_bar_frame];
        let background_color: id = msg_class![env; UIColor whiteColor];
        () = msg![env; tab_bar setBackgroundColor:background_color];
        () = msg![env; root_view addSubview:tab_bar];
    } else {
        retain(env, tab_bar);
        () = msg![env; tab_bar setFrame:tab_bar_frame];
    }
    let host_object = env.objc.borrow_mut::<UITabBarControllerHostObject>(this);
    host_object.tab_bar = tab_bar;
    host_object.view_loaded = true;
    () = msg![env; tab_bar setDelegate:this];

    refresh_tab_bar(env, this);
    show_selected_controller(env, this);
}

- (id)viewControllers {
    let controllers = env.objc.borrow::<UITabBarControllerHostObject>(this).view_controllers.clone();
    for controller in &controllers {
        retain(env, *controller);
    }
    let array = ns_array::from_vec(env, controllers);
    autorelease(env, array)
}

- (())setViewControllers:(id)controllers {
    () = msg![env; this setViewControllers:controllers animated:false];
}

- (())setViewControllers:(id)controllers animated:(bool)_animated {
    let displayed_controller = env
        .objc
        .borrow::<UITabBarControllerHostObject>(this)
        .displayed_controller;
    let old_controllers = std::mem::take(
        &mut env.objc.borrow_mut::<UITabBarControllerHostObject>(this).view_controllers,
    );
    env.objc
        .borrow_mut::<UITabBarControllerHostObject>(this)
        .displayed_controller = nil;
    for controller in old_controllers {
        env.objc.borrow_mut::<UIViewControllerHostObject>(controller).tab_bar_controller = nil;
        if controller == displayed_controller {
            () = msg![env; controller viewWillDisappear:false];
            let view: id = msg![env; controller view];
            () = msg![env; view removeFromSuperview];
            () = msg![env; controller viewDidDisappear:false];
        }
        release(env, controller);
    }

    let count: NSUInteger = msg![env; controllers count];
    let mut new_controllers = Vec::with_capacity(count as usize);
    for index in 0..count {
        let controller: id = msg![env; controllers objectAtIndex:index];
        assert!(!new_controllers.contains(&controller));
        retain(env, controller);
        env.objc.borrow_mut::<UIViewControllerHostObject>(controller).tab_bar_controller = this;
        let item: id = msg![env; controller tabBarItem];
        if item == nil {
            let item: id = msg_class![env; UITabBarItem alloc];
            let title: id = msg![env; controller title];
            let item: id = msg![env; item initWithTitle:title image:nil tag:(index as NSInteger)];
            () = msg![env; controller setTabBarItem:item];
            release(env, item);
        }
        new_controllers.push(controller);
    }
    {
        let host_object = env.objc.borrow_mut::<UITabBarControllerHostObject>(this);
        host_object.view_controllers = new_controllers;
        if host_object.selected_index >= count {
            host_object.selected_index = 0;
        }
    }

    if env.objc.borrow::<UITabBarControllerHostObject>(this).view_loaded {
        refresh_tab_bar(env, this);
        show_selected_controller(env, this);
    }
}

- (NSUInteger)selectedIndex {
    env.objc.borrow::<UITabBarControllerHostObject>(this).selected_index
}
- (())setSelectedIndex:(NSUInteger)index {
    let count = env.objc.borrow::<UITabBarControllerHostObject>(this).view_controllers.len();
    if count > 0 && index >= count as NSUInteger {
        return;
    }
    if env.objc.borrow::<UITabBarControllerHostObject>(this).selected_index == index {
        return;
    }
    env.objc.borrow_mut::<UITabBarControllerHostObject>(this).selected_index = index;
    if env.objc.borrow::<UITabBarControllerHostObject>(this).view_loaded {
        update_tab_bar(env, this);
        show_selected_controller(env, this);
    }
}

- (id)selectedViewController {
    let host_object = env.objc.borrow::<UITabBarControllerHostObject>(this);
    host_object.view_controllers.get(host_object.selected_index as usize).copied().unwrap_or(nil)
}
- (())setSelectedViewController:(id)controller {
    let index = env
        .objc
        .borrow::<UITabBarControllerHostObject>(this)
        .view_controllers
        .iter()
        .position(|&candidate| candidate == controller)
        .expect("selected view controller must belong to the tab bar controller");
    () = msg![env; this setSelectedIndex:(index as NSUInteger)];
}

- (id)tabBar {
    let tab_bar = env.objc.borrow::<UITabBarControllerHostObject>(this).tab_bar;
    if tab_bar == nil {
        let _: id = msg![env; this view];
        env.objc.borrow::<UITabBarControllerHostObject>(this).tab_bar
    } else {
        tab_bar
    }
}
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<UITabBarControllerHostObject>(this).delegate = delegate;
}
- (id)delegate {
    env.objc.borrow::<UITabBarControllerHostObject>(this).delegate
}
- (())_touchHLE_refreshTabBar {
    if env.objc.borrow::<UITabBarControllerHostObject>(this).view_loaded {
        refresh_tab_bar(env, this);
    }
}

- (())tabBar:(id)_tab_bar didSelectItem:(id)item {
    let view_controllers = env
        .objc
        .borrow::<UITabBarControllerHostObject>(this)
        .view_controllers
        .clone();
    let index = view_controllers
        .iter()
        .position(|&controller| {
            let candidate: id = msg![env; controller tabBarItem];
            candidate == item
        });
    if let Some(index) = index {
        () = msg![env; this setSelectedIndex:(index as NSUInteger)];
        let delegate = env.objc.borrow::<UITabBarControllerHostObject>(this).delegate;
        let selector: SEL = env
            .objc
            .register_host_selector(
                "tabBarController:didSelectViewController:".to_string(),
                &mut env.mem,
            );
        let responds: bool = msg![env; delegate respondsToSelector:selector];
        if responds {
            let controller: id = msg![env; this selectedViewController];
            () = msg![env; delegate tabBarController:this didSelectViewController:controller];
        }
    }
}

@end

@implementation UITabBar: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UITabBarHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    let host_object = env.objc.borrow_mut::<UITabBarHostObject>(this);
    let items = std::mem::take(&mut host_object.items);
    let buttons = std::mem::take(&mut host_object.buttons);
    let selected_item = std::mem::replace(&mut host_object.selected_item, nil);
    for button in buttons {
        () = msg![env; button removeFromSuperview];
        release(env, button);
    }
    for item in items {
        release(env, item);
    }
    release(env, selected_item);
    msg_super![env; this dealloc]
}

- (id)items {
    let items = env.objc.borrow::<UITabBarHostObject>(this).items.clone();
    for item in &items {
        retain(env, *item);
    }
    let array = ns_array::from_vec(env, items);
    autorelease(env, array)
}
- (())setItems:(id)items {
    () = msg![env; this setItems:items animated:false];
}
- (())setItems:(id)items animated:(bool)_animated {
    let host_object = env.objc.borrow_mut::<UITabBarHostObject>(this);
    let old_items = std::mem::take(&mut host_object.items);
    let old_buttons = std::mem::take(&mut host_object.buttons);
    let old_selected_item = std::mem::replace(&mut host_object.selected_item, nil);
    for button in old_buttons {
        () = msg![env; button removeFromSuperview];
        release(env, button);
    }
    for item in old_items {
        release(env, item);
    }
    release(env, old_selected_item);

    let count: NSUInteger = msg![env; items count];
    let mut new_items = Vec::with_capacity(count as usize);
    let mut new_buttons = Vec::with_capacity(count as usize);
    for index in 0..count {
        let item: id = msg![env; items objectAtIndex:index];
        retain(env, item);
        new_items.push(item);

        let frame = CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize {
                width: 0.0,
                height: TAB_BAR_HEIGHT,
            },
        };
        let button: id = msg_class![env; UIButton alloc];
        let button: id = msg![env; button initWithFrame:frame];
        let title: id = msg![env; item title];
        if title != nil {
            () = msg![env; button setTitle:title forState:UIControlStateNormal];
        }
        let image: id = msg![env; item image];
        if image != nil {
            () = msg![env; button setImage:image forState:UIControlStateNormal];
        }
        let selector: SEL = env
            .objc
            .lookup_selector("_touchHLE_selectTab:").unwrap();
        () = msg![env; button addTarget:this action:selector forControlEvents:UIControlEventTouchUpInside];
        () = msg![env; button setTag:(index as NSInteger)];
        () = msg![env; this addSubview:button];
        new_buttons.push(button);
    }
    env.objc.borrow_mut::<UITabBarHostObject>(this).items = new_items;
    env.objc.borrow_mut::<UITabBarHostObject>(this).buttons = new_buttons;
    layout_tab_bar(env, this);
}

- (id)selectedItem {
    env.objc.borrow::<UITabBarHostObject>(this).selected_item
}
- (())setSelectedItem:(id)item {
    let host_object = env.objc.borrow::<UITabBarHostObject>(this);
    assert!(item == nil || host_object.items.contains(&item));
    let old_item = host_object.selected_item;
    if old_item == item {
        return;
    }
    retain(env, item);
    env.objc.borrow_mut::<UITabBarHostObject>(this).selected_item = item;
    release(env, old_item);
    let (items, buttons) = {
        let host_object = env.objc.borrow::<UITabBarHostObject>(this);
        (host_object.items.clone(), host_object.buttons.clone())
    };
    for (tab_item, button) in items.iter().zip(buttons.iter()) {
        () = msg![env; (*button) setSelected:(*tab_item == item)];
    }
}
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<UITabBarHostObject>(this).delegate = delegate;
}
- (id)delegate {
    env.objc.borrow::<UITabBarHostObject>(this).delegate
}

- (())_touchHLE_selectTab:(id)button {
    let index: NSInteger = msg![env; button tag];
    let item = env.objc.borrow::<UITabBarHostObject>(this).items[index as usize];
    () = msg![env; this setSelectedItem:item];
    let delegate = env.objc.borrow::<UITabBarHostObject>(this).delegate;
    if delegate != nil {
        let selector: SEL = env
            .objc
            .register_host_selector("tabBar:didSelectItem:".to_string(), &mut env.mem);
        let responds: bool = msg![env; delegate respondsToSelector:selector];
        if responds {
            () = msg![env; delegate tabBar:this didSelectItem:item];
        }
    }
}

- (())setFrame:(CGRect)frame {
    () = msg_super![env; this setFrame:frame];
    layout_tab_bar(env, this);
}

@end

@implementation UITabBarItem: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UITabBarItemHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let title_key = get_static_str(env, "UITitle");
    let title: id = msg![env; coder decodeObjectForKey:title_key];
    let image_key = get_static_str(env, "UIImage");
    let image: id = msg![env; coder decodeObjectForKey:image_key];
    let tag_key = get_static_str(env, "UITag");
    let tag: NSInteger = msg![env; coder decodeIntegerForKey:tag_key];
    msg![env; this initWithTitle:title image:image tag:tag]
}

- (id)initWithTitle:(id)title image:(id)image tag:(NSInteger)tag {
    let host_object = env.objc.borrow_mut::<UITabBarItemHostObject>(this);
    host_object.title = title;
    host_object.image = image;
    host_object.tag = tag;
    retain(env, title);
    retain(env, image);
    this
}
- (id)initWithTabBarSystemItem:(NSInteger)system_item tag:(NSInteger)tag {
    let title = match system_item {
        0 => "More",
        1 => "Favorites",
        2 => "Featured",
        3 => "Top Rated",
        4 => "Recents",
        5 => "Contacts",
        6 => "History",
        7 => "Bookmarks",
        8 => "Search",
        9 => "Downloads",
        10 => "Most Recent",
        11 => "Most Viewed",
        _ => "",
    };
    let title = get_static_str(env, title);
    msg![env; this initWithTitle:title image:nil tag:tag]
}
- (())dealloc {
    let (title, image) = {
        let host_object = env.objc.borrow::<UITabBarItemHostObject>(this);
        (host_object.title, host_object.image)
    };
    release(env, title);
    release(env, image);
    msg_super![env; this dealloc]
}
- (id)title {
    env.objc.borrow::<UITabBarItemHostObject>(this).title
}
- (())setTitle:(id)title {
    retain(env, title);
    let old_title = std::mem::replace(
        &mut env.objc.borrow_mut::<UITabBarItemHostObject>(this).title,
        title,
    );
    release(env, old_title);
}
- (id)image {
    env.objc.borrow::<UITabBarItemHostObject>(this).image
}
- (())setImage:(id)image {
    retain(env, image);
    let old_image = std::mem::replace(
        &mut env.objc.borrow_mut::<UITabBarItemHostObject>(this).image,
        image,
    );
    release(env, old_image);
}
- (NSInteger)tag {
    env.objc.borrow::<UITabBarItemHostObject>(this).tag
}
- (())setTag:(NSInteger)tag {
    env.objc.borrow_mut::<UITabBarItemHostObject>(this).tag = tag;
}

@end

};

fn layout_tab_bar(env: &mut crate::Environment, tab_bar: id) {
    let frame: CGRect = msg![env; tab_bar bounds];
    let buttons = env
        .objc
        .borrow::<UITabBarHostObject>(tab_bar)
        .buttons
        .clone();
    let count = buttons.len();
    if count == 0 {
        return;
    }
    let width = frame.size.width / count as f32;
    for (index, button) in buttons.iter().enumerate() {
        let button_frame = CGRect {
            origin: CGPoint {
                x: index as f32 * width,
                y: 0.0,
            },
            size: CGSize {
                width,
                height: frame.size.height,
            },
        };
        () = msg![env; (*button) setFrame:button_frame];
    }
}

fn update_tab_bar(env: &mut crate::Environment, controller: id) {
    let (tab_bar, selected_controller) = {
        let host_object = env.objc.borrow::<UITabBarControllerHostObject>(controller);
        let selected_controller = host_object
            .view_controllers
            .get(host_object.selected_index as usize)
            .copied()
            .unwrap_or(nil);
        (host_object.tab_bar, selected_controller)
    };
    if tab_bar != nil {
        let selected_item: id = if selected_controller == nil {
            nil
        } else {
            msg![env; selected_controller tabBarItem]
        };
        () = msg![env; tab_bar setSelectedItem:selected_item];
    }
}

fn refresh_tab_bar(env: &mut crate::Environment, controller: id) {
    let (tab_bar, controllers) = {
        let host_object = env.objc.borrow::<UITabBarControllerHostObject>(controller);
        (host_object.tab_bar, host_object.view_controllers.clone())
    };
    if tab_bar == nil {
        return;
    }
    let items = controllers
        .iter()
        .map(|&view_controller| {
            let item: id = msg![env; view_controller tabBarItem];
            retain(env, item);
            item
        })
        .collect();
    let items = ns_array::from_vec(env, items);
    () = msg![env; tab_bar setItems:items];
    release(env, items);
    update_tab_bar(env, controller);
}

fn show_selected_controller(env: &mut crate::Environment, controller: id) {
    let (root_view, selected_controller, tab_bar, displayed_controller, view_loaded) = {
        let host_object = env.objc.borrow::<UITabBarControllerHostObject>(controller);
        (
            host_object.superclass.view,
            host_object
                .view_controllers
                .get(host_object.selected_index as usize)
                .copied()
                .unwrap_or(nil),
            host_object.tab_bar,
            host_object.displayed_controller,
            host_object.view_loaded,
        )
    };
    if !view_loaded || root_view == nil {
        return;
    }
    if displayed_controller != nil && displayed_controller != selected_controller {
        () = msg![env; displayed_controller viewWillDisappear:false];
        let old_view: id = msg![env; displayed_controller view];
        () = msg![env; old_view removeFromSuperview];
        () = msg![env; displayed_controller viewDidDisappear:false];
    }
    if selected_controller != nil {
        let view: id = msg![env; selected_controller view];
        let root_bounds: CGRect = msg![env; root_view bounds];
        let tab_bar_height = if tab_bar == nil { 0.0 } else { TAB_BAR_HEIGHT };
        let content_frame = CGRect {
            origin: CGPoint {
                x: root_bounds.origin.x,
                y: root_bounds.origin.y,
            },
            size: CGSize {
                width: root_bounds.size.width,
                height: (root_bounds.size.height - tab_bar_height).max(0.0),
            },
        };
        () = msg![env; selected_controller viewWillAppear:false];
        () = msg![env; view setFrame:content_frame];
        () = msg![env; root_view addSubview:view];
        () = msg![env; selected_controller viewDidAppear:false];
    }
    env.objc
        .borrow_mut::<UITabBarControllerHostObject>(controller)
        .displayed_controller = selected_controller;
    if tab_bar != nil {
        () = msg![env; root_view bringSubviewToFront:tab_bar];
    }
}
