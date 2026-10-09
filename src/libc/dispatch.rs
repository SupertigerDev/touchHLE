/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Grand Central Dispatch functions used by apps.

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::mem::{ConstVoidPtr, MutPtr, SafeRead};
use crate::objc::{id, msg_class};
use crate::Environment;
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    main_queue: id,
    global_queues: HashMap<i32, id>,
    block_refcounts: HashMap<ConstVoidPtr, u32>,
}

fn get_main_queue(env: &mut Environment) -> id {
    if env.libc_state.dispatch.main_queue == id::null() {
        let queue = msg_class![env; NSObject new];
        env.libc_state.dispatch.main_queue = queue;
    }
    env.libc_state.dispatch.main_queue
}

fn main_queue_constant(env: &mut Environment) -> ConstVoidPtr {
    get_main_queue(env).cast().cast_const()
}

#[repr(C, packed)]
pub struct BlockLiteral {
    pub isa: ConstVoidPtr,
    pub flags: i32,
    pub reserved: i32,
    pub invoke: GuestFunction,
    pub descriptor: ConstVoidPtr,
}
unsafe impl SafeRead for BlockLiteral {}

#[repr(C, packed)]
struct BlockDescriptor {
    reserved: u32,
    size: u32,
    copy_helper: GuestFunction,
    dispose_helper: GuestFunction,
}
unsafe impl SafeRead for BlockDescriptor {}

const BLOCK_NEEDS_FREE: u32 = 1 << 24;
const BLOCK_HAS_COPY_DISPOSE: u32 = 1 << 25;
const BLOCK_IS_GLOBAL: u32 = 1 << 28;
const BLOCK_HAS_SIGNATURE: u32 = 1 << 30;
const BLOCK_HAS_EXTENDED_LAYOUT: u32 = 1 << 31;

fn concrete_block_class(env: &mut Environment) -> ConstVoidPtr {
    let class: id = msg_class![env; NSObject class];
    class.cast().cast_const()
}

pub fn is_block(env: &Environment, object: id) -> bool {
    if object == id::null() {
        return false;
    }
    let block = env.mem.read(object.cast::<BlockLiteral>());
    let flags = block.flags as u32;
    let block_flags =
        BLOCK_HAS_COPY_DISPOSE | BLOCK_IS_GLOBAL | BLOCK_HAS_SIGNATURE | BLOCK_HAS_EXTENDED_LAYOUT;
    if flags & block_flags == 0 || block.descriptor.is_null() || block.invoke.to_ptr().is_null() {
        return false;
    }
    let descriptor = env.mem.read(block.descriptor.cast::<BlockDescriptor>());
    descriptor.size >= std::mem::size_of::<BlockLiteral>() as u32
}

pub fn copy_block(env: &mut Environment, object: id) -> id {
    if !is_block(env, object) {
        return object;
    }
    let block = env.mem.read(object.cast::<BlockLiteral>());
    let flags = block.flags as u32;
    if flags & BLOCK_IS_GLOBAL != 0 {
        return object;
    }

    let block_key = object.cast_const().cast();
    if flags & BLOCK_NEEDS_FREE != 0 {
        let refcount = env
            .libc_state
            .dispatch
            .block_refcounts
            .entry(block_key)
            .or_insert((flags & 0xffff).max(1));
        *refcount += 1;
        let mut block = block;
        block.flags = ((flags & !0xffff) | *refcount) as i32;
        env.mem.write(object.cast(), block);
        return object.cast();
    }

    let descriptor = env.mem.read(block.descriptor.cast::<BlockDescriptor>());
    let descriptor_size = descriptor.size;
    let copy_helper = descriptor.copy_helper;
    let heap_block = env.mem.alloc(descriptor_size).cast::<BlockLiteral>();
    env.mem.memmove(
        heap_block.cast(),
        object.cast_const().cast(),
        descriptor_size,
    );
    let mut copied_block = block;
    copied_block.flags = ((flags | BLOCK_NEEDS_FREE) & !0xffff | 1) as i32;
    env.mem.write(heap_block, copied_block);
    env.libc_state
        .dispatch
        .block_refcounts
        .insert(heap_block.cast_const().cast(), 1);

    if flags & BLOCK_HAS_COPY_DISPOSE != 0 {
        assert!(!copy_helper.to_ptr().is_null());
        () = copy_helper.call_from_host(
            env,
            (
                heap_block.cast::<std::ffi::c_void>().cast_const(),
                object.cast::<std::ffi::c_void>().cast_const(),
            ),
        );
    }
    heap_block.cast()
}

pub fn release_block(env: &mut Environment, object: id) {
    if !is_block(env, object) {
        return;
    }
    let block_key = object.cast_const().cast();
    let block = env.mem.read(object.cast::<BlockLiteral>());
    let flags = block.flags as u32;
    if flags & BLOCK_NEEDS_FREE == 0 {
        return;
    }

    let refcount = env
        .libc_state
        .dispatch
        .block_refcounts
        .get_mut(&block_key)
        .expect("Releasing a heap block not created by the block runtime");
    *refcount -= 1;
    if *refcount != 0 {
        let mut block = block;
        block.flags = ((flags & !0xffff) | *refcount) as i32;
        env.mem.write(object.cast(), block);
        return;
    }

    env.libc_state.dispatch.block_refcounts.remove(&block_key);
    if flags & BLOCK_HAS_COPY_DISPOSE != 0 {
        let descriptor = env.mem.read(block.descriptor.cast::<BlockDescriptor>());
        let dispose_helper = descriptor.dispose_helper;
        assert!(!dispose_helper.to_ptr().is_null());
        () = dispose_helper.call_from_host(env, (object.cast::<std::ffi::c_void>().cast_const(),));
    }
}

fn _Block_copy(env: &mut Environment, object: id) -> id {
    copy_block(env, object)
}

fn _Block_release(env: &mut Environment, object: id) {
    release_block(env, object)
}

fn dispatch_once(env: &mut Environment, predicate: MutPtr<u32>, block: ConstVoidPtr) {
    if env.mem.read(predicate) == 0 {
        let invoke = env.mem.read(block.cast::<BlockLiteral>()).invoke;
        assert!(!invoke.to_ptr().is_null());
        env.mem.write(predicate, 1);
        () = invoke.call_from_host(env, (block,));
    }
}

fn dispatch_once_f(
    env: &mut Environment,
    predicate: MutPtr<u32>,
    context: ConstVoidPtr,
    function: GuestFunction,
) {
    if env.mem.read(predicate) == 0 {
        env.mem.write(predicate, 1);
        () = function.call_from_host(env, (context,));
    }
}

fn dispatch_get_main_queue(env: &mut Environment) -> id {
    get_main_queue(env)
}

fn dispatch_get_global_queue(env: &mut Environment, identifier: i32, _flags: u32) -> id {
    if let Some(queue) = env.libc_state.dispatch.global_queues.get(&identifier) {
        return *queue;
    }
    let queue = msg_class![env; NSObject new];
    env.libc_state
        .dispatch
        .global_queues
        .insert(identifier, queue);
    queue
}

fn dispatch_async(env: &mut Environment, _queue: id, block: ConstVoidPtr) {
    let invoke = env.mem.read(block.cast::<BlockLiteral>()).invoke;
    assert!(!invoke.to_ptr().is_null());
    log!("TODO: dispatch_async executes blocks synchronously");
    () = invoke.call_from_host(env, (block,));
}

pub const CONSTANTS: ConstantExports = &[
    (
        "__dispatch_main_q",
        HostConstant::Custom(main_queue_constant),
    ),
    (
        "__NSConcreteStackBlock",
        HostConstant::Custom(concrete_block_class),
    ),
    (
        "__NSConcreteGlobalBlock",
        HostConstant::Custom(concrete_block_class),
    ),
];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(dispatch_once(_, _)),
    export_c_func!(dispatch_once_f(_, _, _)),
    export_c_func!(dispatch_get_main_queue()),
    export_c_func!(dispatch_get_global_queue(_, _)),
    export_c_func!(dispatch_async(_, _)),
    export_c_func!(_Block_copy(_)),
    export_c_func!(_Block_release(_)),
];
