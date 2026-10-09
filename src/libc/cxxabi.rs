/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `cxxabi.h`
//!
//! Resources:
//! - [Itanium C++ ABI specification](https://itanium-cxx-abi.github.io/cxx-abi/abi.html#dso-dtor-runtime-api)

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, export_c_func_aliased, FunctionExports};
use crate::mem::MutVoidPtr;
use crate::Environment;

fn __cxa_atexit(
    _env: &mut Environment,
    func: GuestFunction, // void (*func)(void *)
    p: MutVoidPtr,
    d: MutVoidPtr,
) -> i32 {
    // TODO: when this is implemented, make sure it's properly compatible with
    // C atexit.
    log!(
        "TODO: __cxa_atexit({:?}, {:?}, {:?}) (unimplemented)",
        func,
        p,
        d
    );
    0 // success
}

fn __cxa_finalize(_env: &mut Environment, d: MutVoidPtr) {
    log!("TODO: __cxa_finalize({:?}) (unimplemented)", d);
}

fn __Unwind_SjLj_Register(_env: &mut Environment, function_context: MutVoidPtr) {
    log_dbg!(
        "TODO: __Unwind_SjLj_Register({:?}) (exception unwinding unimplemented)",
        function_context
    );
}

fn __Unwind_SjLj_Unregister(_env: &mut Environment, function_context: MutVoidPtr) {
    log_dbg!(
        "TODO: __Unwind_SjLj_Unregister({:?}) (exception unwinding unimplemented)",
        function_context
    );
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(__cxa_atexit(_, _, _)),
    export_c_func!(__cxa_finalize(_)),
    export_c_func_aliased!("_Unwind_SjLj_Register", __Unwind_SjLj_Register(_)),
    export_c_func_aliased!("_Unwind_SjLj_Unregister", __Unwind_SjLj_Unregister(_)),
];
