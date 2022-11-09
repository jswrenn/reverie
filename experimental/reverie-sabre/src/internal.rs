/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under the BSD-style license found in the
 * LICENSE file in the root directory of this source tree.
 */

//! Everything defined here shouldn't be used directly. However, they must be
//! exposed so that code generated by the proc macros can use them.

#![doc(hidden)]
pub use ::once_cell::sync::OnceCell;
use reverie_rpc::MakeClient;

use super::callbacks;
use super::ffi;
use super::paths;
use super::rpc;
use super::signal;
use super::tool::Tool;
use super::tool::ToolGlobal;

/// Creates an instance of a Tool. This is called when `ToolGlobal::global` is
/// called for the first time.
pub fn init_tool<T: Tool>() -> T {
    // Create the base transport channel. This transport layer can be wrapped
    // potentially many times by nested tools. If this fails (i.e., it failed to
    // connect to the socket), there isn't anything we can do except panic. A
    // client without a connection to the global state isn't very useful.
    let channel = rpc::BaseChannel::new().unwrap();

    T::new(MakeClient::make_client(Box::new(channel)))
}

fn register_detours<T: ToolGlobal>(fn_icept_reg: ffi::icept_reg_fn) {
    for detour_func in <<T as ToolGlobal>::Target>::detours() {
        fn_icept_reg(detour_func);
    }
}

pub fn sbr_init<T: ToolGlobal>(
    argc: *mut i32,
    argv: *mut *mut *mut libc::c_char,
    fn_icept_reg: ffi::icept_reg_fn,
    vdso_callback: *mut Option<ffi::handle_vdso_fn>,
    syscall_handler: *mut Option<ffi::handle_syscall_fn>,
    rdtsc_handler: *mut Option<ffi::handle_rdtsc_fn>,
    _post_load: *mut Option<ffi::post_load_fn>,
    sabre_path: *const libc::c_char,
    client_path: *const libc::c_char,
) {
    unsafe {
        *vdso_callback = Some(callbacks::handle_vdso::<T>);
        *syscall_handler = Some(callbacks::handle_syscall::<T>);
        *rdtsc_handler = Some(callbacks::handle_rdtsc::<T>);

        paths::set_sabre_path(sabre_path);
        paths::set_client_path(client_path);

        // The plugin path is the first argument.
        paths::set_plugin_path(**argv);

        signal::register_central_handler::<T>();

        *argc -= 1;
        *argv = (*argv).wrapping_add(1);

        // Setting up function detours
        register_detours::<T>(fn_icept_reg);
    }
}
