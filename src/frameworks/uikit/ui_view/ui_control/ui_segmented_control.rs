/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISegmentedControl`.

use crate::frameworks::core_graphics::CGRect;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg_super, nil, objc_classes, release, retain,
    ClassExports, NSZonePtr,
};

struct UISegmentedControlHostObject {
    superclass: super::UIControlHostObject,
    selected_segment_index: NSInteger,
    titles: Vec<id>,
}
impl_HostObject_with_superclass!(UISegmentedControlHostObject);

impl Default for UISegmentedControlHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            selected_segment_index: -1,
            titles: Vec::new(),
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISegmentedControl: UIControl

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(this, Box::<UISegmentedControlHostObject>::default(), &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    log!("[(UISegmentedControl*){:?} initWithFrame:{:?}] TODO: Implement UISegmentedControl. The control won't be rendered.", this, frame);
    msg_super![env; this initWithFrame:frame]
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log!("[(UISegmentedControl*){:?} initWithCoder:{:?}] TODO: Implement UISegmentedControl. The control won't be rendered.", this, coder);
    msg_super![env; this initWithCoder:coder]
}

- (())dealloc {
    let titles = std::mem::take(
        &mut env.objc.borrow_mut::<UISegmentedControlHostObject>(this).titles,
    );
    for title in titles {
        release(env, title);
    }
    msg_super![env; this dealloc]
}

- (NSInteger)selectedSegmentIndex {
    env.objc
        .borrow::<UISegmentedControlHostObject>(this)
        .selected_segment_index
}
- (())setSelectedSegmentIndex:(NSInteger)index {
    env.objc
        .borrow_mut::<UISegmentedControlHostObject>(this)
        .selected_segment_index = index;
}

- (NSUInteger)numberOfSegments {
    env.objc
        .borrow::<UISegmentedControlHostObject>(this)
        .titles
        .len() as NSUInteger
}
- (())setTitle:(id)title forSegmentAtIndex:(NSUInteger)index {
    retain(env, title);
    let titles = &mut env
        .objc
        .borrow_mut::<UISegmentedControlHostObject>(this)
        .titles;
    titles.resize_with(index as usize + 1, || nil);
    let old_title = std::mem::replace(&mut titles[index as usize], title);
    release(env, old_title);
}
- (id)titleForSegmentAtIndex:(NSUInteger)index {
    env.objc
        .borrow::<UISegmentedControlHostObject>(this)
        .titles
        .get(index as usize)
        .copied()
        .unwrap_or(nil)
}

// TODO: all of it

@end

// Undocumented class used by UISegmentedControl
@implementation UISegment: UIControl

- (id)initWithFrame:(CGRect)frame {
    log!("[(UISegment*){:?} initWithFrame:{:?}] Attempted to initialize undocumented class from outside of the NIB.", this, frame);
    unreachable!()
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    log!("[(UISegment*){:?} initWithCoder:{:?}] TODO: Implement UISegment. The control won't be rendered.", this, coder);
    msg_super![env; this initWithCoder:coder]
}

// TODO: all of it

@end

};
