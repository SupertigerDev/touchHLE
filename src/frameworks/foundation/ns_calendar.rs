/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSCalendar` and `NSDateComponents`.
//!
//! Only the Gregorian calendar in the system time zone is supported, and
//! `components:fromDate:` always fills in year, month, day, hour, minute and
//! second regardless of the requested units.

use super::{NSInteger, NSTimeInterval, NSUInteger};
use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_foundation::time::CFAbsoluteTimeGetGregorianDate;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, ClassExports, HostObject, NSZonePtr,
};

const NS_UNDEFINED_DATE_COMPONENT: NSInteger = 0x7fffffff;
const NS_GREGORIAN_CALENDAR: &str = "gregorian";

pub const CONSTANTS: ConstantExports = &[(
    "_NSGregorianCalendar",
    HostConstant::NSString(NS_GREGORIAN_CALENDAR),
)];

struct NSCalendarHostObject;
impl HostObject for NSCalendarHostObject {}

struct NSDateComponentsHostObject {
    year: NSInteger,
    month: NSInteger,
    day: NSInteger,
    hour: NSInteger,
    minute: NSInteger,
    second: NSInteger,
}
impl HostObject for NSDateComponentsHostObject {}
impl Default for NSDateComponentsHostObject {
    fn default() -> Self {
        NSDateComponentsHostObject {
            year: NS_UNDEFINED_DATE_COMPONENT,
            month: NS_UNDEFINED_DATE_COMPONENT,
            day: NS_UNDEFINED_DATE_COMPONENT,
            hour: NS_UNDEFINED_DATE_COMPONENT,
            minute: NS_UNDEFINED_DATE_COMPONENT,
            second: NS_UNDEFINED_DATE_COMPONENT,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSCalendar: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(this, Box::new(NSCalendarHostObject), &mut env.mem)
}

+ (id)currentCalendar {
    let new: id = msg![env; this alloc];
    autorelease(env, new)
}

+ (id)autoupdatingCurrentCalendar {
    msg![env; this currentCalendar]
}

- (id)initWithCalendarIdentifier:(id)calendar_identifier {
    let gregorian_identifier = crate::frameworks::foundation::ns_string::get_static_str(
        env,
        NS_GREGORIAN_CALENDAR,
    );
    let is_gregorian: bool =
        msg![env; calendar_identifier isEqualToString:gregorian_identifier];
    assert!(is_gregorian, "Only the Gregorian calendar is supported");
    this
}

- (id)components:(NSUInteger)_unit_flags
         fromDate:(id)date { // NSDate*
    let ti: NSTimeInterval = msg![env; date timeIntervalSinceReferenceDate];
    let greg = CFAbsoluteTimeGetGregorianDate(env, ti, nil);
    let components: id = msg_class![env; NSDateComponents new];
    let host = env.objc.borrow_mut::<NSDateComponentsHostObject>(components);
    host.year = greg.year;
    host.month = greg.month.into();
    host.day = greg.day.into();
    host.hour = greg.hours.into();
    host.minute = greg.minutes.into();
    host.second = greg.seconds as NSInteger;
    autorelease(env, components)
}

@end

@implementation NSDateComponents: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<NSDateComponentsHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (NSInteger)year {
    env.objc.borrow::<NSDateComponentsHostObject>(this).year
}
- (())setYear:(NSInteger)value {
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).year = value;
}
- (NSInteger)month {
    env.objc.borrow::<NSDateComponentsHostObject>(this).month
}
- (())setMonth:(NSInteger)value {
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).month = value;
}
- (NSInteger)day {
    env.objc.borrow::<NSDateComponentsHostObject>(this).day
}
- (())setDay:(NSInteger)value {
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).day = value;
}
- (NSInteger)hour {
    env.objc.borrow::<NSDateComponentsHostObject>(this).hour
}
- (())setHour:(NSInteger)value {
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).hour = value;
}
- (NSInteger)minute {
    env.objc.borrow::<NSDateComponentsHostObject>(this).minute
}
- (())setMinute:(NSInteger)value {
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).minute = value;
}
- (NSInteger)second {
    env.objc.borrow::<NSDateComponentsHostObject>(this).second
}
- (())setSecond:(NSInteger)value {
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).second = value;
}

@end

};
