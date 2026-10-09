/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Objective-C runtime.
//!
//! Apple's [Programming with Objective-C](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ProgrammingWithObjectiveC/Introduction/Introduction.html)
//! is a useful introduction to the language from a user's perspective.
//! There are further resources in the child modules of this module, but they
//! are more implementation-specific.
//!
//! The strategy for this emulator will be to provide our own implementations of
//! an Objective-C runtime and libraries for it (Foundation etc). These
//! implementations will be "host code": Rust code forming part of the emulator,
//! not emulated code. The runtime will need to be able to handle classes that
//! originate from the guest app, classes defined by the host, and sometimes
//! classes that are both (considering Objective-C's support for inheritance,
//! categories and dynamic class editing).

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant, HostDylib};
use crate::objc::messages::ThreadInitializer;
use crate::MutexId;
use std::collections::HashMap;

mod classes;
mod messages;
mod methods;
mod objects;
mod properties;
mod selectors;
mod synchronization;

pub use classes::{objc_classes, Class, ClassExports, ClassTemplate};
pub use messages::{
    autorelease, msg, msg_class, msg_send, msg_send_no_initialize, msg_send_no_type_checking,
    msg_send_super2, msg_super, objc_super, release, retain,
};
pub use methods::{HostIMP, IMP};
pub use objects::{
    id, impl_HostObject_with_superclass, nil, AnyHostObject, HostObject, TrivialHostObject,
};
pub use properties::todo_objc_setter;
pub use selectors::{selector, SEL};

use crate::mem::{ConstVoidPtr, GuestISize, MutVoidPtr};
use crate::Environment;
pub(crate) use classes::class_getMethodImplementation;
use classes::{
    class_getInstanceSize, class_getProperty, class_getSuperclass, class_replaceMethod,
    objc_getClass, ClassHostObject, FakeClass, UnimplementedClass,
};
pub(crate) use messages::objc_msgSend;
use messages::{objc_msgSendSuper2, objc_msgSend_stret, MsgSendSignature, MsgSendSuperSignature};
use methods::method_list_t;
use objects::{objc_object, object_getClass, HostObjectEntry};
use properties::{ivar_list_t, objc_copyStruct, objc_getProperty, objc_setProperty};
use selectors::sel_registerName;
use synchronization::{objc_sync_enter, objc_sync_exit};

/// Typedef for `NSZone *`. This is a [fossil type] found in the signature of
/// `allocWithZone:` and similar methods. Its value is always ignored.
///
/// [fossil type]: https://en.wiktionary.org/wiki/fossil_word
pub type NSZonePtr = crate::mem::MutVoidPtr;

/// Main type holding Objective-C runtime state.
pub struct ObjC {
    /// Known selectors (interned method name strings).
    selectors: HashMap<String, SEL>,

    /// Mapping of known (guest) object pointers to their host objects.
    ///
    /// If an object isn't in this map, we will consider it not to exist.
    objects: HashMap<id, HostObjectEntry>,

    /// Known classes.
    ///
    /// Look at the `isa` to get the metaclass for a class.
    classes: HashMap<String, Class>,

    /// Guest memory locations containing non-retaining object references.
    weak_references: HashMap<MutVoidPtr, id>,

    /// Mutexes used in @synchronized blocks (objc_sync_enter/exit).
    sync_mutexes: HashMap<id, MutexId>,

    /// Mutexes for running the +initialize function.
    initializer_threads: HashMap<id, ThreadInitializer>,

    /// Temporary storage for optional type information when sending a message.
    /// Type information isn't part of the `objc_msgSend` ABI, so an alternative
    /// channel is needed.
    message_type_info: Option<(std::any::TypeId, &'static str)>,
}

impl ObjC {
    pub fn new() -> ObjC {
        ObjC {
            selectors: HashMap::new(),
            objects: HashMap::new(),
            classes: HashMap::new(),
            weak_references: HashMap::new(),
            sync_mutexes: HashMap::new(),
            initializer_threads: HashMap::new(),
            message_type_info: None,
        }
    }
}

pub const DYLIB: HostDylib = HostDylib {
    path: "/usr/lib/libobjc.A.dylib",
    aliases: &["/usr/lib/libobjc.dylib"],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[FUNCTIONS],
};

const CONSTANTS: ConstantExports = &[
    // We don't use these in our Objective-C runtime, but exporting useless
    // symbols for these silences the warning about the unhandled relocation,
    // and avoids a linker error for the integration tests.
    ("__objc_empty_vtable", HostConstant::NullPtr),
    ("__objc_empty_cache", HostConstant::NullPtr),
];

/// Block support is iOS 4+, but it seems like Block Runtime Helpers
/// could still be called on even if minimal iOS version is set to 3.x?
///
/// ref. <https://clang.llvm.org/docs/Block-ABI-Apple.html#runtime-helper-functions>
fn _Block_object_assign(
    env: &mut Environment,
    destination: MutVoidPtr,
    object: ConstVoidPtr,
    flags: i32,
) {
    let object_type = flags & 0xf;
    let object = object.cast_mut().cast();
    let value = match object_type {
        3 => {
            if flags & 0x10 == 0 {
                objc_retain(env, object)
            } else {
                object
            }
        }
        7 => crate::libc::dispatch::copy_block(env, object),
        8 => {
            log!("Warning: Ignoring _Block_object_assign for __block variable");
            object
        }
        _ => panic!("Unsupported _Block_object_assign flags {flags:#x}"),
    };
    env.mem.write(destination.cast(), value);
}

fn _Block_object_dispose(env: &mut Environment, object: ConstVoidPtr, flags: i32) {
    let object_type = flags & 0xf;
    match object_type {
        3 if flags & 0x10 == 0 => objc_release(env, object.cast_mut().cast()),
        3 => {}
        7 => crate::libc::dispatch::release_block(env, object.cast_mut().cast()),
        8 => {
            log!("Warning: Ignoring _Block_object_dispose for __block variable");
        }
        _ => panic!("Unsupported _Block_object_dispose flags {flags:#x}"),
    }
}

fn objc_storeWeak(env: &mut Environment, location: MutVoidPtr, object: id) -> id {
    assert!(!location.is_null());
    env.objc.weak_references.insert(location, object);
    env.mem.write(location.cast(), object);
    object
}

fn objc_storeStrong(env: &mut Environment, location: MutVoidPtr, object: id) {
    assert!(!location.is_null());
    objc_retain(env, object);
    let old_object: id = env.mem.read(location.cast());
    env.mem.write(location.cast(), object);
    objc_release(env, old_object);
}

fn objc_setProperty_atomic(
    env: &mut Environment,
    object: id,
    selector: SEL,
    value: id,
    offset: GuestISize,
) {
    objc_setProperty(env, object, selector, offset, value, true, 0)
}

fn objc_setProperty_nonatomic(
    env: &mut Environment,
    object: id,
    selector: SEL,
    value: id,
    offset: GuestISize,
) {
    objc_setProperty(env, object, selector, offset, value, false, 0)
}

fn objc_setProperty_atomic_copy(
    env: &mut Environment,
    object: id,
    selector: SEL,
    value: id,
    offset: GuestISize,
) {
    objc_setProperty(env, object, selector, offset, value, true, 1)
}

fn objc_setProperty_nonatomic_copy(
    env: &mut Environment,
    object: id,
    selector: SEL,
    value: id,
    offset: GuestISize,
) {
    objc_setProperty(env, object, selector, offset, value, false, 1)
}

fn objc_retain(env: &mut Environment, object: id) -> id {
    if crate::libc::dispatch::is_block(env, object) {
        return crate::libc::dispatch::copy_block(env, object);
    }
    retain(env, object)
}

fn objc_release(env: &mut Environment, object: id) {
    if crate::libc::dispatch::is_block(env, object) {
        crate::libc::dispatch::release_block(env, object);
        return;
    }
    release(env, object)
}

fn objc_autorelease(env: &mut Environment, object: id) -> id {
    if crate::libc::dispatch::is_block(env, object) {
        return crate::libc::dispatch::copy_block(env, object);
    }
    autorelease(env, object)
}

fn objc_retainAutoreleasedReturnValue(env: &mut Environment, object: id) -> id {
    objc_retain(env, object)
}

fn objc_retainAutorelease(env: &mut Environment, object: id) -> id {
    let object = objc_retain(env, object);
    objc_autorelease(env, object)
}

fn objc_retainAutoreleaseReturnValue(env: &mut Environment, object: id) -> id {
    let object = objc_retain(env, object);
    objc_autorelease(env, object)
}

fn objc_autoreleaseReturnValue(env: &mut Environment, object: id) -> id {
    objc_autorelease(env, object)
}

fn objc_unsafeClaimAutoreleasedReturnValue(_env: &mut Environment, object: id) -> id {
    object
}

fn objc_initWeak(env: &mut Environment, location: MutVoidPtr, object: id) -> id {
    objc_storeWeak(env, location, object)
}

fn objc_loadWeakRetained(env: &mut Environment, location: MutVoidPtr) -> id {
    assert!(!location.is_null());
    let object: id = env.mem.read(location.cast());
    retain(env, object);
    object
}

fn objc_loadWeak(env: &mut Environment, location: MutVoidPtr) -> id {
    let object = objc_loadWeakRetained(env, location);
    autorelease(env, object)
}

fn objc_destroyWeak(env: &mut Environment, location: MutVoidPtr) {
    assert!(!location.is_null());
    env.objc.weak_references.remove(&location);
    env.mem.write(location.cast(), nil);
}

fn objc_copyWeak(env: &mut Environment, destination: MutVoidPtr, source: MutVoidPtr) {
    assert!(!source.is_null());
    let object: id = env.mem.read(source.cast());
    objc_storeWeak(env, destination, object);
}

fn objc_moveWeak(env: &mut Environment, destination: MutVoidPtr, source: MutVoidPtr) {
    assert!(!source.is_null());
    let object: id = env.mem.read(source.cast());
    objc_destroyWeak(env, source);
    objc_storeWeak(env, destination, object);
}

const FUNCTIONS: FunctionExports = &[
    export_c_func!(class_getInstanceSize(_)),
    export_c_func!(class_getSuperclass(_)),
    export_c_func!(class_getProperty(_, _)),
    export_c_func!(class_getMethodImplementation(_, _)),
    export_c_func!(class_replaceMethod(_, _, _, _)),
    export_c_func!(objc_msgSend(_, _)),
    export_c_func!(objc_msgSend_stret(_, _, _)),
    export_c_func!(objc_msgSendSuper2(_, _)),
    export_c_func!(objc_getClass(_)),
    export_c_func!(objc_getProperty(_, _, _, _)),
    export_c_func!(objc_setProperty(_, _, _, _, _, _)),
    export_c_func!(objc_setProperty_atomic(_, _, _, _)),
    export_c_func!(objc_setProperty_nonatomic(_, _, _, _)),
    export_c_func!(objc_setProperty_atomic_copy(_, _, _, _)),
    export_c_func!(objc_setProperty_nonatomic_copy(_, _, _, _)),
    export_c_func!(objc_copyStruct(_, _, _, _, _)),
    export_c_func!(objc_sync_enter(_)),
    export_c_func!(objc_sync_exit(_)),
    export_c_func!(object_getClass(_)),
    export_c_func!(sel_registerName(_)),
    export_c_func!(_Block_object_assign(_, _, _)),
    export_c_func!(_Block_object_dispose(_, _)),
    export_c_func!(objc_retain(_)),
    export_c_func!(objc_release(_)),
    export_c_func!(objc_autorelease(_)),
    export_c_func!(objc_retainAutoreleasedReturnValue(_)),
    export_c_func!(objc_retainAutorelease(_)),
    export_c_func!(objc_retainAutoreleaseReturnValue(_)),
    export_c_func!(objc_autoreleaseReturnValue(_)),
    export_c_func!(objc_unsafeClaimAutoreleasedReturnValue(_)),
    export_c_func!(objc_storeStrong(_, _)),
    export_c_func!(objc_storeWeak(_, _)),
    export_c_func!(objc_initWeak(_, _)),
    export_c_func!(objc_loadWeakRetained(_)),
    export_c_func!(objc_loadWeak(_)),
    export_c_func!(objc_destroyWeak(_)),
    export_c_func!(objc_copyWeak(_, _)),
    export_c_func!(objc_moveWeak(_, _)),
];
