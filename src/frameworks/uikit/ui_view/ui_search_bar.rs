/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISearchBar`.

use crate::frameworks::foundation::NSInteger;
use crate::impl_HostObject_with_superclass;
use crate::objc::{
    id, msg_super, nil, objc_classes, release, retain, todo_objc_setter, ClassExports, NSZonePtr,
};

#[derive(Default)]
struct UISearchBarHostObject {
    superclass: super::UIViewHostObject,
    /// `NSString*`
    text: id,
    /// `NSString*`
    placeholder: id,
    /// Weak/non-retaining
    delegate: id,
}
impl_HostObject_with_superclass!(UISearchBarHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// TODO: actually draw the search field and send delegate messages. For now
// this only stores state so that apps containing a search bar can run.
@implementation UISearchBar: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UISearchBarHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    let &UISearchBarHostObject {
        superclass: _,
        text,
        placeholder,
        delegate: _,
    } = env.objc.borrow(this);
    release(env, text);
    release(env, placeholder);
    msg_super![env; this dealloc]
}

- (id)text {
    env.objc.borrow::<UISearchBarHostObject>(this).text
}
- (())setText:(id)text { // NSString*
    retain(env, text);
    let old = std::mem::replace(
        &mut env.objc.borrow_mut::<UISearchBarHostObject>(this).text,
        text,
    );
    release(env, old);
}

- (id)placeholder {
    env.objc.borrow::<UISearchBarHostObject>(this).placeholder
}
- (())setPlaceholder:(id)placeholder { // NSString*
    retain(env, placeholder);
    let old = std::mem::replace(
        &mut env.objc.borrow_mut::<UISearchBarHostObject>(this).placeholder,
        placeholder,
    );
    release(env, old);
}

- (id)delegate {
    env.objc.borrow::<UISearchBarHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<UISearchBarHostObject>(this).delegate = delegate;
}

- (())setBarStyle:(NSInteger)style {
    todo_objc_setter!(this, style);
}
- (())setTintColor:(id)color { // UIColor*
    todo_objc_setter!(this, color);
}
- (())setShowsCancelButton:(bool)shows {
    todo_objc_setter!(this, shows);
}
- (())setShowsCancelButton:(bool)shows
                  animated:(bool)_animated {
    todo_objc_setter!(this, shows);
}
- (())setKeyboardType:(NSInteger)type_ {
    todo_objc_setter!(this, type_);
}
- (())setAutocapitalizationType:(NSInteger)type_ {
    todo_objc_setter!(this, type_);
}
- (())setAutocorrectionType:(NSInteger)type_ {
    todo_objc_setter!(this, type_);
}
- (())setSearchBarStyle:(NSInteger)style {
    todo_objc_setter!(this, style);
}

@end

// TODO: actually present search results. This only stores references so that
// apps using a search display controller can run.
@implementation UISearchDisplayController: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UISearchDisplayControllerHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)_coder {
    this
}

- (id)initWithSearchBar:(id)search_bar // UISearchBar*
     contentsController:(id)contents_controller { // UIViewController*
    let host = env.objc.borrow_mut::<UISearchDisplayControllerHostObject>(this);
    host.search_bar = search_bar;
    host.contents_controller = contents_controller;
    this
}

- (id)searchBar {
    env.objc.borrow::<UISearchDisplayControllerHostObject>(this).search_bar
}
- (())setSearchBar:(id)search_bar {
    env.objc.borrow_mut::<UISearchDisplayControllerHostObject>(this).search_bar = search_bar;
}
- (())setSearchContentsController:(id)controller {
    env.objc.borrow_mut::<UISearchDisplayControllerHostObject>(this).contents_controller = controller;
}
- (id)searchContentsController {
    env.objc.borrow::<UISearchDisplayControllerHostObject>(this).contents_controller
}
- (id)searchResultsTableView {
    nil
}
- (id)delegate {
    env.objc.borrow::<UISearchDisplayControllerHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<UISearchDisplayControllerHostObject>(this).delegate = delegate;
}
- (())setSearchResultsDataSource:(id)data_source {
    todo_objc_setter!(this, data_source);
}
- (())setSearchResultsDelegate:(id)delegate {
    todo_objc_setter!(this, delegate);
}
- (())setActive:(bool)active {
    todo_objc_setter!(this, active);
}
- (())setActive:(bool)active
       animated:(bool)_animated {
    todo_objc_setter!(this, active);
}

@end

};

#[derive(Default)]
struct UISearchDisplayControllerHostObject {
    /// Weak/non-retaining
    search_bar: id,
    /// Weak/non-retaining
    contents_controller: id,
    /// Weak/non-retaining
    delegate: id,
}
impl crate::objc::HostObject for UISearchDisplayControllerHostObject {}
