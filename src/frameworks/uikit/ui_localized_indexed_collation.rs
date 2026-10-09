/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UILocalizedIndexedCollation`.
//!
//! Only an English-style collation (A-Z and "#") is implemented.

use crate::frameworks::foundation::ns_string::{from_rust_string, to_rust_string};
use crate::frameworks::foundation::{ns_array, NSInteger, NSUInteger};
use crate::objc::{autorelease, id, msg, nil, objc_classes, ClassExports, SEL};
use crate::Environment;

fn section_titles(env: &mut Environment) -> id {
    let mut titles: Vec<id> = ('A'..='Z')
        .map(|c| from_rust_string(env, c.to_string()))
        .collect();
    titles.push(from_rust_string(env, "#".to_string()));
    ns_array::from_vec(env, titles)
}

fn section_for_string(env: &mut Environment, string: id) -> NSInteger {
    if string == nil {
        return 26;
    }
    let string = to_rust_string(env, string);
    match string.chars().next().map(|c| c.to_ascii_uppercase()) {
        Some(c @ 'A'..='Z') => (c as u8 - b'A') as NSInteger,
        _ => 26,
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UILocalizedIndexedCollation: NSObject

+ (id)currentCollation {
    let new: id = msg![env; this new];
    autorelease(env, new)
}

- (id)sectionTitles {
    section_titles(env)
}

- (id)sectionIndexTitles {
    section_titles(env)
}

- (NSInteger)sectionForSectionIndexTitleAtIndex:(NSInteger)index {
    index
}

- (NSInteger)sectionForObject:(id)object
      collationStringSelector:(SEL)selector {
    let string: id = msg![env; object performSelector:selector];
    section_for_string(env, string)
}

- (id)sortedArrayFromArray:(id)array // NSArray*
   collationStringSelector:(SEL)selector {
    let count: NSUInteger = msg![env; array count];
    let mut keyed: Vec<(String, id)> = Vec::new();
    for i in 0..count {
        let object: id = msg![env; array objectAtIndex:i];
        let string: id = msg![env; object performSelector:selector];
        let key = if string == nil {
            String::new()
        } else {
            to_rust_string(env, string).to_lowercase()
        };
        keyed.push((key, object));
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    let objects: Vec<id> = keyed.into_iter().map(|(_, o)| o).collect();
    ns_array::from_vec(env, objects)
}

@end

};
