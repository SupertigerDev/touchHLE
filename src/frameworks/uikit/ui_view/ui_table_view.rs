/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Basic `UITableView` and `UITableViewCell` support.

use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::frameworks::uikit::ui_view::{ui_scroll_view::UIScrollViewHostObject, UIViewHostObject};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_super, nil, objc_classes, release, retain,
    ClassExports, NSZonePtr,
};

struct UITableViewHostObject {
    superclass: UIScrollViewHostObject,
    data_source: id,
    row_height: f32,
    separator_style: NSInteger,
    allows_selection: bool,
}
impl_HostObject_with_superclass!(UITableViewHostObject);
impl Default for UITableViewHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            data_source: nil,
            row_height: 44.0,
            separator_style: 1,
            allows_selection: true,
        }
    }
}

struct UITableViewCellHostObject {
    superclass: UIViewHostObject,
    reuse_identifier: id,
    selection_style: NSInteger,
    accessory_type: NSInteger,
}
impl_HostObject_with_superclass!(UITableViewCellHostObject);
impl Default for UITableViewCellHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            reuse_identifier: nil,
            selection_style: 0,
            accessory_type: 0,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITableView: UIScrollView

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc
        .alloc_object(this, Box::<UITableViewHostObject>::default(), &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];

    let key = get_static_str(env, "UIRowHeight");
    let row_height: f32 = msg![env; coder decodeFloatForKey:key];
    if row_height > 0.0 {
        env.objc.borrow_mut::<UITableViewHostObject>(this).row_height = row_height;
    }

    let key = get_static_str(env, "UISeparatorStyle");
    let separator_style: NSInteger = msg![env; coder decodeIntegerForKey:key];
    env.objc.borrow_mut::<UITableViewHostObject>(this).separator_style = separator_style;

    let key = get_static_str(env, "UIAllowSelectingCells");
    let allows_selection: bool = msg![env; coder decodeBoolForKey:key];
    env.objc.borrow_mut::<UITableViewHostObject>(this).allows_selection = allows_selection;

    this
}

- (())dealloc {
    let data_source = env
        .objc
        .borrow_mut::<UITableViewHostObject>(this)
        .data_source;
    release(env, data_source);
    msg_super![env; this dealloc]
}

- (id)dataSource {
    env.objc.borrow::<UITableViewHostObject>(this).data_source
}
- (())setDataSource:(id)data_source {
    env.objc
        .borrow_mut::<UITableViewHostObject>(this)
        .data_source = data_source;
}

- (f32)rowHeight {
    env.objc.borrow::<UITableViewHostObject>(this).row_height
}
- (())setRowHeight:(f32)row_height {
    env.objc.borrow_mut::<UITableViewHostObject>(this).row_height = row_height;
}

- (NSInteger)separatorStyle {
    env.objc.borrow::<UITableViewHostObject>(this).separator_style
}
- (())setSeparatorStyle:(NSInteger)separator_style {
    env.objc
        .borrow_mut::<UITableViewHostObject>(this)
        .separator_style = separator_style;
}

- (bool)allowsSelection {
    env.objc.borrow::<UITableViewHostObject>(this).allows_selection
}
- (())setAllowsSelection:(bool)allows_selection {
    env.objc
        .borrow_mut::<UITableViewHostObject>(this)
        .allows_selection = allows_selection;
}

- (())reloadData {
    log!("TODO: [(UITableView*){:?} reloadData]", this);
}

- (NSUInteger)numberOfSections {
    let data_source: id = msg![env; this dataSource];
    let selector = env
        .objc
        .register_host_selector("numberOfSectionsInTableView:".to_string(), &mut env.mem);
    if msg![env; data_source respondsToSelector:selector] {
        msg![env; data_source numberOfSectionsInTableView:this]
    } else {
        1
    }
}

- (NSUInteger)numberOfRowsInSection:(NSInteger)section {
    let data_source: id = msg![env; this dataSource];
    let selector = env
        .objc
        .register_host_selector("tableView:numberOfRowsInSection:".to_string(), &mut env.mem);
    if msg![env; data_source respondsToSelector:selector] {
        msg![env; data_source tableView:this numberOfRowsInSection:section]
    } else {
        0
    }
}

- (id)cellForRowAtIndexPath:(id)index_path {
    let data_source: id = msg![env; this dataSource];
    let selector = env
        .objc
        .register_host_selector("tableView:cellForRowAtIndexPath:".to_string(), &mut env.mem);
    if msg![env; data_source respondsToSelector:selector] {
        msg![env; data_source tableView:this cellForRowAtIndexPath:index_path]
    } else {
        nil
    }
}

- (id)dequeueReusableCellWithIdentifier:(id)_identifier {
    nil
}

- (id)indexPathForSelectedRow {
    nil
}

@end

@implementation UITableViewCell: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc
        .alloc_object(this, Box::<UITableViewCellHostObject>::default(), &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    msg_super![env; this initWithCoder:coder]
}

- (id)initWithStyle:(NSInteger)_style reuseIdentifier:(id)reuse_identifier {
    let this: id = msg_super![env; this init];
    let host_object = env.objc.borrow_mut::<UITableViewCellHostObject>(this);
    host_object.reuse_identifier = reuse_identifier;
    retain(env, reuse_identifier);
    this
}

- (())dealloc {
    let reuse_identifier = env
        .objc
        .borrow_mut::<UITableViewCellHostObject>(this)
        .reuse_identifier;
    release(env, reuse_identifier);
    msg_super![env; this dealloc]
}

- (id)reuseIdentifier {
    env.objc
        .borrow::<UITableViewCellHostObject>(this)
        .reuse_identifier
}

- (id)contentView {
    this
}

- (id)textLabel {
    nil
}

- (id)detailTextLabel {
    nil
}

- (NSInteger)selectionStyle {
    env.objc
        .borrow::<UITableViewCellHostObject>(this)
        .selection_style
}
- (())setSelectionStyle:(NSInteger)selection_style {
    env.objc
        .borrow_mut::<UITableViewCellHostObject>(this)
        .selection_style = selection_style;
}

- (NSInteger)accessoryType {
    env.objc
        .borrow::<UITableViewCellHostObject>(this)
        .accessory_type
}
- (())setAccessoryType:(NSInteger)accessory_type {
    env.objc
        .borrow_mut::<UITableViewCellHostObject>(this)
        .accessory_type = accessory_type;
}

- (())setSelected:(bool)_selected animated:(bool)_animated {}
- (())setHighlighted:(bool)_highlighted animated:(bool)_animated {}

@end

@implementation UITableViewCellContentView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc
        .alloc_object(this, Box::<UIViewHostObject>::default(), &mut env.mem)
}

@end

};
