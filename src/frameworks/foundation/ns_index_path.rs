/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSIndexPath`.

use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::objc::{
    autorelease, id, msg, objc_classes, ClassExports, HostObject, NSZonePtr,
};

#[derive(Default)]
struct NSIndexPathHostObject {
    indexes: Vec<NSUInteger>,
}
impl HostObject for NSIndexPathHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// Only the two-index (section, row) form used by UITableView is supported.
@implementation NSIndexPath: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<NSIndexPathHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)indexPathForRow:(NSInteger)row inSection:(NSInteger)section {
    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<NSIndexPathHostObject>(new).indexes =
        vec![section as NSUInteger, row as NSUInteger];
    autorelease(env, new)
}

+ (id)indexPathForItem:(NSInteger)item inSection:(NSInteger)section {
    msg![env; this indexPathForRow:item inSection:section]
}

+ (id)indexPathWithIndex:(NSUInteger)index {
    let new: id = msg![env; this alloc];
    env.objc.borrow_mut::<NSIndexPathHostObject>(new).indexes = vec![index];
    autorelease(env, new)
}

- (NSInteger)section {
    env.objc.borrow::<NSIndexPathHostObject>(this).indexes.first().copied().unwrap_or(0) as NSInteger
}
- (NSInteger)row {
    env.objc.borrow::<NSIndexPathHostObject>(this).indexes.get(1).copied().unwrap_or(0) as NSInteger
}
- (NSInteger)item {
    env.objc.borrow::<NSIndexPathHostObject>(this).indexes.get(1).copied().unwrap_or(0) as NSInteger
}
- (NSUInteger)length {
    env.objc.borrow::<NSIndexPathHostObject>(this).indexes.len() as NSUInteger
}
- (NSUInteger)indexAtPosition:(NSUInteger)position {
    env.objc.borrow::<NSIndexPathHostObject>(this).indexes.get(position as usize).copied().unwrap_or(0)
}

- (id)copyWithZone:(NSZonePtr)_zone {
    let indexes = env.objc.borrow::<NSIndexPathHostObject>(this).indexes.clone();
    let class: id = msg![env; this class];
    let new: id = msg![env; class alloc];
    env.objc.borrow_mut::<NSIndexPathHostObject>(new).indexes = indexes;
    new
}

- (bool)isEqual:(id)other {
    if this == other {
        return true;
    }
    let class: id = msg![env; this class];
    let is_same_class: bool = msg![env; other isKindOfClass:class];
    if !is_same_class {
        return false;
    }
    env.objc.borrow::<NSIndexPathHostObject>(this).indexes
        == env.objc.borrow::<NSIndexPathHostObject>(other).indexes
}

- (NSUInteger)hash {
    let indexes = &env.objc.borrow::<NSIndexPathHostObject>(this).indexes;
    indexes.iter().fold(17, |h, &i| h.wrapping_mul(31).wrapping_add(i))
}

@end

};
