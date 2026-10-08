/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `dlfcn.h` (`dlopen()` and friends)

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, ConstVoidPtr, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;

const RTLD_DEFAULT: MutVoidPtr = Ptr::from_bits(-2 as _);

#[repr(C)]
struct DlInfo {
    dli_fname: ConstPtr<u8>,
    dli_fbase: ConstVoidPtr,
    dli_sname: ConstPtr<u8>,
    dli_saddr: ConstVoidPtr,
}

unsafe impl crate::mem::SafeRead for DlInfo {}

fn is_known_library(path: &str) -> bool {
    crate::dyld::DYLIB_LIST
        .iter()
        .any(|dylib| dylib.path == path || dylib.aliases.contains(&path))
}

fn dlopen(env: &mut Environment, path: ConstPtr<u8>, _mode: i32) -> MutVoidPtr {
    if path.is_null() {
        return RTLD_DEFAULT;
    }
    // TODO: dlopen() support for real dynamic libraries.
    assert!(is_known_library(env.mem.cstr_at_utf8(path).unwrap()));
    // For convenience, use the path as the handle.
    // TODO: Find out whether the handle is truly opaque on iPhone OS, and if
    // not, where it points.
    path.cast_mut().cast()
}

fn dlsym(env: &mut Environment, handle: MutVoidPtr, symbol: ConstPtr<u8>) -> MutVoidPtr {
    assert!(
        handle == RTLD_DEFAULT || is_known_library(env.mem.cstr_at_utf8(handle.cast()).unwrap())
    );
    // For some reason, the symbols passed to dlsym() don't have the leading _.
    let symbol = format!("_{}", env.mem.cstr_at_utf8(symbol).unwrap());
    // TODO: error handling. dlsym() should just return NULL in this case, but
    // currently it's probably more useful to have the emulator crash if there's
    // no symbol found, since it most likely indicates a missing host function.
    // TODO: Symbol lookup should be scoped to the specific library requested,
    // where appropriate!
    let addr = env
        .dyld
        .create_proc_address(&mut env.mem, &mut env.cpu, &symbol)
        .unwrap_or_else(|_| panic!("dlsym() for unimplemented function {symbol}"));
    Ptr::from_bits(addr.addr_with_thumb_bit())
}

fn dlclose(env: &mut Environment, handle: MutVoidPtr) -> i32 {
    assert!(
        handle == RTLD_DEFAULT || is_known_library(env.mem.cstr_at_utf8(handle.cast()).unwrap())
    );
    0 // success
}

fn dladdr(env: &mut Environment, address: ConstVoidPtr, info: MutPtr<DlInfo>) -> i32 {
    let address = address.to_bits() & !1;
    let Some(bin) = env.bins.iter().find(|bin| {
        bin.loaded_segments
            .iter()
            .any(|range| range.contains(&address))
    }) else {
        return 0;
    };

    let nearest_symbol = bin
        .exported_symbols
        .iter()
        .filter_map(|(name, &symbol_address)| {
            let symbol_address_without_thumb_bit = symbol_address & !1;
            (symbol_address_without_thumb_bit <= address).then_some((
                name.as_str(),
                symbol_address,
                symbol_address_without_thumb_bit,
            ))
        })
        .max_by_key(|&(_, _, symbol_address)| symbol_address);

    let (dli_sname, dli_saddr) = nearest_symbol.map_or(
        (ConstPtr::null(), ConstVoidPtr::null()),
        |(name, symbol_address, _)| {
            (
                env.mem.alloc_and_write_cstr(name.as_bytes()).cast_const(),
                Ptr::from_bits(symbol_address),
            )
        },
    );
    env.mem.write(
        info,
        DlInfo {
            dli_fname: bin.guest_path.cast_const(),
            dli_fbase: Ptr::from_bits(bin.image_base),
            dli_sname,
            dli_saddr,
        },
    );
    1
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(dlopen(_, _)),
    export_c_func!(dlsym(_, _)),
    export_c_func!(dlclose(_)),
    export_c_func!(dladdr(_, _)),
];
