/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPickerView`.

use std::collections::HashMap;

use crate::frameworks::foundation::NSInteger;
use crate::frameworks::uikit::ui_view::UIViewHostObject;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_super, nil, objc_classes, release, retain,
    ClassExports, NSZonePtr,
};

// TODO: rendering

struct UIPickerViewHostObject {
    superclass: UIViewHostObject,
    data_source: id,
    delegate: id,
    selected_rows: HashMap<NSInteger, NSInteger>,
    shows_selection_indicator: bool,
}
impl_HostObject_with_superclass!(UIPickerViewHostObject);
impl Default for UIPickerViewHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            data_source: nil,
            delegate: nil,
            selected_rows: HashMap::new(),
            shows_selection_indicator: false,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIPickerView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc
        .alloc_object(this, Box::<UIPickerViewHostObject>::default(), &mut env.mem)
}

- (())dealloc {
    let &UIPickerViewHostObject {
        superclass: _,
        data_source,
        delegate,
        ..
    } = env.objc.borrow(this);
    release(env, data_source);
    release(env, delegate);
    msg_super![env; this dealloc]
}

- (())setShowsSelectionIndicator:(bool)shows {
    env.objc
        .borrow_mut::<UIPickerViewHostObject>(this)
        .shows_selection_indicator = shows;
}
- (bool)showsSelectionIndicator {
    env.objc
        .borrow::<UIPickerViewHostObject>(this)
        .shows_selection_indicator
}
- (id)dataSource {
    env.objc.borrow::<UIPickerViewHostObject>(this).data_source
}
- (())setDataSource:(id)data_source {
    retain(env, data_source);
    let old_data_source = std::mem::replace(
        &mut env.objc.borrow_mut::<UIPickerViewHostObject>(this).data_source,
        data_source,
    );
    release(env, old_data_source);
}
- (NSInteger)numberOfComponents {
    let data_source = env.objc.borrow::<UIPickerViewHostObject>(this).data_source;
    msg![env; data_source numberOfComponentsInPickerView:this]
}
- (NSInteger)numberOfRowsInComponent:(NSInteger)component {
    let data_source = env.objc.borrow::<UIPickerViewHostObject>(this).data_source;
    msg![env; data_source pickerView:this numberOfRowsInComponent:component]
}
- (())selectRow:(NSInteger)row
   inComponent:(NSInteger)component
       animated:(bool)_animated {
    env.objc
        .borrow_mut::<UIPickerViewHostObject>(this)
        .selected_rows
        .insert(component, row);
}
- (NSInteger)selectedRowInComponent:(NSInteger)component {
    env.objc
        .borrow::<UIPickerViewHostObject>(this)
        .selected_rows
        .get(&component)
        .copied()
        .unwrap_or(-1)
}
- (())reloadAllComponents {
    log!("TODO: UIPickerView rendering");
}
- (())reloadComponent:(NSInteger)_component {
    log!("TODO: UIPickerView rendering");
}
- (id)delegate {
    env.objc.borrow::<UIPickerViewHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    retain(env, delegate);
    let old_delegate = std::mem::replace(
        &mut env.objc.borrow_mut::<UIPickerViewHostObject>(this).delegate,
        delegate,
    );
    release(env, old_delegate);
}

@end

};
