#![cfg(windows)]

extern crate winapi;

use winapi::shared::{
    minwindef,
    minwindef::{BOOL, DWORD, HINSTANCE, LPVOID}
};

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern "system" fn DllMain(
    dll_module: HINSTANCE,
    call_reason: DWORD,
    reserved: LPVOID)
    -> BOOL
{
    const DLL_PROCESS_ATTACH: DWORD = 1;
    const DLL_PROCESS_DETACH: DWORD = 0;

    match call_reason {
        DLL_PROCESS_ATTACH => demo_init(),
        DLL_PROCESS_DETACH => (),
        _ => ()
    }
    minwindef::TRUE
}

fn demo_init() {
}
