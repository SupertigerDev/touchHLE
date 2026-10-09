/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Basic `UITableView` and `UITableViewCell` support.

use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::frameworks::uikit::ui_view::{ui_scroll_view::UIScrollViewHostObject, UIViewHostObject};
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    retain, ClassExports, NSZonePtr,
};
use crate::todo_objc_setter;

const SECTION_HEADER_HEIGHT: CGFloat = 22.0;

struct UITableViewHostObject {
    superclass: UIScrollViewHostObject,
    data_source: id,
    row_height: f32,
    separator_style: NSInteger,
    allows_selection: bool,
    /// Retained cells currently laid out, with their (section, row).
    cells: Vec<(id, NSInteger, NSInteger)>,
    /// Retained section header views.
    header_views: Vec<id>,
    touch_start_offset: CGPoint,
    editing: bool,
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
            cells: Vec::new(),
            header_views: Vec::new(),
            touch_start_offset: CGPoint { x: 0.0, y: 0.0 },
            editing: false,
        }
    }
}

struct UITableViewCellHostObject {
    superclass: UIViewHostObject,
    reuse_identifier: id,
    selection_style: NSInteger,
    accessory_type: NSInteger,
    /// Owned by the cell's subview list, so non-retaining here.
    text_label: id,
    detail_text_label: id,
}
impl_HostObject_with_superclass!(UITableViewCellHostObject);
impl Default for UITableViewCellHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            reuse_identifier: nil,
            selection_style: 0,
            accessory_type: 0,
            text_label: nil,
            detail_text_label: nil,
        }
    }
}

fn responds_to(env: &mut crate::Environment, object: id, selector: &str) -> bool {
    let sel = env
        .objc
        .register_host_selector(selector.to_string(), &mut env.mem);
    msg![env; object respondsToSelector:sel]
}

/// Creates a transparent `UILabel` and adds it to `cell`.
fn make_cell_label(env: &mut crate::Environment, cell: id, font_size: CGFloat) -> id {
    let label: id = msg_class![env; UILabel alloc];
    let label: id = msg![env; label initWithFrame:(CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize { width: 0.0, height: 0.0 },
    })];
    let clear: id = msg_class![env; UIColor clearColor];
    () = msg![env; label setBackgroundColor:clear];
    let font: id = msg_class![env; UIFont systemFontOfSize:font_size];
    () = msg![env; label setFont:font];
    () = msg![env; cell addSubview:label];
    release(env, label);
    label
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
    let host = env.objc.borrow_mut::<UITableViewHostObject>(this);
    let data_source = host.data_source;
    let cells = std::mem::take(&mut host.cells);
    let header_views = std::mem::take(&mut host.header_views);
    release(env, data_source);
    for (cell, _, _) in cells {
        release(env, cell);
    }
    for view in header_views {
        release(env, view);
    }
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

// Simplified: every cell is created up front and stacked vertically,
// there is no cell reuse or lazy loading.
- (())reloadData {
    let host = env.objc.borrow_mut::<UITableViewHostObject>(this);
    let old_cells = std::mem::take(&mut host.cells);
    let old_headers = std::mem::take(&mut host.header_views);
    let default_row_height = host.row_height;
    for (cell, _, _) in old_cells {
        () = msg![env; cell removeFromSuperview];
        release(env, cell);
    }
    for view in old_headers {
        () = msg![env; view removeFromSuperview];
        release(env, view);
    }

    let bounds: CGRect = msg![env; this bounds];
    let width = bounds.size.width;
    let data_source: id = msg![env; this dataSource];
    let delegate: id = msg![env; this delegate];
    let has_titles = responds_to(env, data_source, "tableView:titleForHeaderInSection:");
    let has_heights = responds_to(env, delegate, "tableView:heightForRowAtIndexPath:");

    let section_count: NSUInteger = msg![env; this numberOfSections];
    let mut y: CGFloat = 0.0;
    for section in 0..section_count as NSInteger {
        if has_titles {
            let title: id = msg![env; data_source tableView:this titleForHeaderInSection:section];
            if title != nil {
                let header: id = msg_class![env; UILabel alloc];
                let header: id = msg![env; header initWithFrame:(CGRect {
                    origin: CGPoint { x: 0.0, y },
                    size: CGSize { width, height: SECTION_HEADER_HEIGHT },
                })];
                let background: id = msg_class![env; UIColor grayColor];
                () = msg![env; header setBackgroundColor:background];
                let text_color: id = msg_class![env; UIColor whiteColor];
                () = msg![env; header setTextColor:text_color];
                let font: id = msg_class![env; UIFont systemFontOfSize:(14.0 as CGFloat)];
                () = msg![env; header setFont:font];
                () = msg![env; header setText:title];
                () = msg![env; this addSubview:header];
                env.objc.borrow_mut::<UITableViewHostObject>(this).header_views.push(header);
                y += SECTION_HEADER_HEIGHT;
            }
        }

        let row_count: NSUInteger = msg![env; this numberOfRowsInSection:section];
        for row in 0..row_count as NSInteger {
            let index_path: id = msg_class![env; NSIndexPath indexPathForRow:row inSection:section];
            let cell: id = msg![env; this cellForRowAtIndexPath:index_path];
            if cell == nil {
                continue;
            }
            let height: CGFloat = if has_heights {
                msg![env; delegate tableView:this heightForRowAtIndexPath:index_path]
            } else {
                default_row_height
            };
            () = msg![env; cell setFrame:(CGRect {
                origin: CGPoint { x: 0.0, y },
                size: CGSize { width, height },
            })];
            () = msg![env; cell layoutSubviews];
            () = msg![env; this addSubview:cell];
            retain(env, cell);
            env.objc
                .borrow_mut::<UITableViewHostObject>(this)
                .cells
                .push((cell, section, row));
            y += height;
        }
    }

    () = msg![env; this setContentSize:(CGSize { width, height: y })];
    () = msg![env; this setNeedsDisplay];
}

- (())touchesBegan:(id)touches // NSSet* of UITouch*
         withEvent:(id)event { // UIEvent*
    let offset: CGPoint = msg![env; this contentOffset];
    env.objc.borrow_mut::<UITableViewHostObject>(this).touch_start_offset = offset;
    msg_super![env; this touchesBegan:touches withEvent:event]
}

- (())touchesEnded:(id)touches // NSSet* of UITouch*
         withEvent:(id)event { // UIEvent*
    let start_offset = env.objc.borrow::<UITableViewHostObject>(this).touch_start_offset;
    let offset: CGPoint = msg![env; this contentOffset];
    let allows_selection = env.objc.borrow::<UITableViewHostObject>(this).allows_selection;
    // Only treat it as a tap if the table didn't scroll.
    if allows_selection && offset == start_offset {
        let touch_arr: id = msg![env; touches allObjects];
        let touch: id = msg![env; touch_arr objectAtIndex:0u32];
        let mut view: id = msg![env; touch view];
        let mut found = None;
        while view != nil && view != this {
            let host = env.objc.borrow::<UITableViewHostObject>(this);
            if let Some(&(_, section, row)) = host.cells.iter().find(|&&(c, _, _)| c == view) {
                found = Some((section, row));
                break;
            }
            view = msg![env; view superview];
        }
        if let Some((section, row)) = found {
            let delegate: id = msg![env; this delegate];
            if responds_to(env, delegate, "tableView:didSelectRowAtIndexPath:") {
                let index_path: id = msg_class![env; NSIndexPath indexPathForRow:row inSection:section];
                () = msg![env; delegate tableView:this didSelectRowAtIndexPath:index_path];
            }
        }
    }
    msg_super![env; this touchesEnded:touches withEvent:event]
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

- (())registerNib:(id)_nib // UINib*
forCellReuseIdentifier:(id)_identifier { // NSString*
    log!("TODO: [(UITableView*){:?} registerNib:forCellReuseIdentifier:]", this);
}

- (())registerClass:(id)_class
forCellReuseIdentifier:(id)_identifier { // NSString*
    log!("TODO: [(UITableView*){:?} registerClass:forCellReuseIdentifier:]", this);
}

- (id)indexPathForSelectedRow {
    nil
}

- (())scrollToRowAtIndexPath:(id)_index_path
            atScrollPosition:(NSInteger)_position
                    animated:(bool)_animated {
    log!("TODO: [(UITableView*){:?} scrollToRowAtIndexPath:atScrollPosition:animated:]", this);
}
- (())selectRowAtIndexPath:(id)_index_path
                  animated:(bool)_animated
            scrollPosition:(NSInteger)_position {
    log!("TODO: [(UITableView*){:?} selectRowAtIndexPath:animated:scrollPosition:]", this);
}
- (())deselectRowAtIndexPath:(id)_index_path
                    animated:(bool)_animated {
    log!("TODO: [(UITableView*){:?} deselectRowAtIndexPath:animated:]", this);
}
- (())beginUpdates {
    log!("TODO: [(UITableView*){:?} beginUpdates]", this);
}
- (())endUpdates {
    log!("TODO: [(UITableView*){:?} endUpdates]", this);
}
- (())insertRowsAtIndexPaths:(id)_paths // NSArray*
            withRowAnimation:(NSInteger)_animation {
    msg![env; this reloadData]
}
- (())deleteRowsAtIndexPaths:(id)_paths // NSArray*
            withRowAnimation:(NSInteger)_animation {
    msg![env; this reloadData]
}
- (())reloadRowsAtIndexPaths:(id)_paths // NSArray*
            withRowAnimation:(NSInteger)_animation {
    msg![env; this reloadData]
}
- (())reloadSections:(id)_sections // NSIndexSet*
    withRowAnimation:(NSInteger)_animation {
    msg![env; this reloadData]
}
- (())setSectionIndexMinimumDisplayRowCount:(NSInteger)count {
    todo_objc_setter!(this, count);
}
- (())setTableHeaderView:(id)view { // UIView*
    todo_objc_setter!(this, view);
}
- (())setTableFooterView:(id)view { // UIView*
    todo_objc_setter!(this, view);
}
- (())setEditing:(bool)editing {
    env.objc.borrow_mut::<UITableViewHostObject>(this).editing = editing;
}
- (())setEditing:(bool)editing
        animated:(bool)_animated {
    env.objc.borrow_mut::<UITableViewHostObject>(this).editing = editing;
}
- (bool)isEditing {
    env.objc.borrow::<UITableViewHostObject>(this).editing
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
    let white: id = msg_class![env; UIColor whiteColor];
    () = msg![env; this setBackgroundColor:white];
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
    let label = env.objc.borrow::<UITableViewCellHostObject>(this).text_label;
    if label != nil {
        return label;
    }
    let label = make_cell_label(env, this, 17.0);
    env.objc.borrow_mut::<UITableViewCellHostObject>(this).text_label = label;
    label
}

- (id)detailTextLabel {
    let label = env.objc.borrow::<UITableViewCellHostObject>(this).detail_text_label;
    if label != nil {
        return label;
    }
    let label = make_cell_label(env, this, 14.0);
    let gray: id = msg_class![env; UIColor grayColor];
    () = msg![env; label setTextColor:gray];
    env.objc.borrow_mut::<UITableViewCellHostObject>(this).detail_text_label = label;
    label
}

- (())setText:(id)text { // NSString*
    let label: id = msg![env; this textLabel];
    msg![env; label setText:text]
}

- (())layoutSubviews {
    let bounds: CGRect = msg![env; this bounds];
    let host = env.objc.borrow::<UITableViewCellHostObject>(this);
    let (text_label, detail_label, accessory) =
        (host.text_label, host.detail_text_label, host.accessory_type);
    let right_inset: CGFloat = if accessory != 0 { 40.0 } else { 10.0 };
    let width = (bounds.size.width - 10.0 - right_inset).max(0.0);
    if text_label != nil {
        let frame = if detail_label != nil {
            CGRect {
                origin: CGPoint { x: 10.0, y: 0.0 },
                size: CGSize { width: width / 2.0, height: bounds.size.height },
            }
        } else {
            CGRect {
                origin: CGPoint { x: 10.0, y: 0.0 },
                size: CGSize { width, height: bounds.size.height },
            }
        };
        () = msg![env; text_label setFrame:frame];
    }
    if detail_label != nil {
        () = msg![env; detail_label setFrame:(CGRect {
            origin: CGPoint { x: 10.0 + width / 2.0, y: 0.0 },
            size: CGSize { width: width / 2.0, height: bounds.size.height },
        })];
    }
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
