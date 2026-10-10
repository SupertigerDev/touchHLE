/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSNumberFormatter`.

use crate::frameworks::foundation::ns_string::{from_rust_string, to_rust_string};
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, todo_objc_setter, ClassExports, HostObject,
    NSZonePtr,
};

struct NSNumberFormatterHostObject {
    number_style: NSInteger,
    minimum_fraction_digits: NSUInteger,
    maximum_fraction_digits: NSUInteger,
    minimum_integer_digits: NSUInteger,
    uses_grouping_separator: bool,
}
impl HostObject for NSNumberFormatterHostObject {}
impl Default for NSNumberFormatterHostObject {
    fn default() -> Self {
        Self {
            number_style: 0,
            minimum_fraction_digits: 0,
            maximum_fraction_digits: 3,
            minimum_integer_digits: 1,
            uses_grouping_separator: false,
        }
    }
}

fn format_number(value: f64, formatter: &NSNumberFormatterHostObject) -> String {
    let precision = formatter
        .maximum_fraction_digits
        .max(formatter.minimum_fraction_digits) as usize;
    let formatted = format!("{value:.precision$}");
    let (integer, fraction) = formatted.split_once('.').unwrap_or((&formatted, ""));
    let fraction = fraction.trim_end_matches('0');
    let fraction_length = fraction
        .len()
        .max(formatter.minimum_fraction_digits as usize);
    let integer = if integer.len() < formatter.minimum_integer_digits as usize {
        format!(
            "{}{}",
            "0".repeat(formatter.minimum_integer_digits as usize - integer.len()),
            integer
        )
    } else {
        integer.to_string()
    };
    let integer = if formatter.uses_grouping_separator {
        let (sign, digits) = integer
            .strip_prefix('-')
            .map_or(("", integer.as_str()), |digits| ("-", digits));
        let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
        for (index, digit) in digits.chars().rev().enumerate() {
            if index != 0 && index % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(digit);
        }
        format!("{sign}{}", grouped.chars().rev().collect::<String>())
    } else {
        integer
    };
    if fraction_length == 0 {
        integer
    } else {
        let fraction = format!(
            "{fraction}{}",
            "0".repeat(fraction_length.saturating_sub(fraction.len()))
        );
        format!("{integer}.{fraction}")
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSNumberFormatter: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(
        this,
        Box::<NSNumberFormatterHostObject>::default(),
        &mut env.mem,
    )
}

- (())setNumberStyle:(NSInteger)number_style {
    env.objc
        .borrow_mut::<NSNumberFormatterHostObject>(this)
        .number_style = number_style;
}
- (NSInteger)numberStyle {
    env.objc
        .borrow::<NSNumberFormatterHostObject>(this)
        .number_style
}
- (())setMinimumFractionDigits:(NSUInteger)digits {
    env.objc
        .borrow_mut::<NSNumberFormatterHostObject>(this)
        .minimum_fraction_digits = digits;
}
- (NSUInteger)minimumFractionDigits {
    env.objc
        .borrow::<NSNumberFormatterHostObject>(this)
        .minimum_fraction_digits
}
- (())setMaximumFractionDigits:(NSUInteger)digits {
    env.objc
        .borrow_mut::<NSNumberFormatterHostObject>(this)
        .maximum_fraction_digits = digits;
}
- (NSUInteger)maximumFractionDigits {
    env.objc
        .borrow::<NSNumberFormatterHostObject>(this)
        .maximum_fraction_digits
}
- (())setMinimumIntegerDigits:(NSUInteger)digits {
    env.objc
        .borrow_mut::<NSNumberFormatterHostObject>(this)
        .minimum_integer_digits = digits;
}
- (NSUInteger)minimumIntegerDigits {
    env.objc
        .borrow::<NSNumberFormatterHostObject>(this)
        .minimum_integer_digits
}
- (())setUsesGroupingSeparator:(bool)uses_grouping_separator {
    env.objc
        .borrow_mut::<NSNumberFormatterHostObject>(this)
        .uses_grouping_separator = uses_grouping_separator;
}
- (bool)usesGroupingSeparator {
    env.objc
        .borrow::<NSNumberFormatterHostObject>(this)
        .uses_grouping_separator
}
- (())setLocale:(id)_locale {
    todo_objc_setter!(this, _locale);
}
- (id)stringFromNumber:(id)number {
    if number == nil {
        return nil;
    }
    let value: f64 = msg![env; number doubleValue];
    let formatted = {
        let formatter = env.objc.borrow::<NSNumberFormatterHostObject>(this);
        format_number(value, formatter)
    };
    let result = from_rust_string(env, formatted);
    autorelease(env, result)
}
- (id)numberFromString:(id)string {
    if string == nil {
        return nil;
    }
    let value = to_rust_string(env, string).replace(',', "");
    match value.parse::<f64>() {
        Ok(value) => msg_class![env; NSNumber numberWithDouble:value],
        Err(error) => {
            log!("NSNumberFormatter could not parse {value:?}: {error}");
            nil
        }
    }
}

@end

};
