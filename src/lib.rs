#![cfg(windows)]
extern crate winapi;
#[macro_use]
extern crate detour;

use std::{thread, sync::Mutex, rc::Rc};
use winapi::{
    um::{
        winnt::LPCWSTR,
        winhttp::{HINTERNET, INTERNET_PORT, WinHttpConnect}
    },
    shared::{
        minwindef,
        minwindef::{BOOL, DWORD, HINSTANCE, LPVOID},
    }
};

mod server;

static_detours! {
    struct DetourConnect: unsafe extern "system" fn(HINTERNET, LPCWSTR, INTERNET_PORT, DWORD) -> HINTERNET;
}

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
        DLL_PROCESS_ATTACH => init(),
        DLL_PROCESS_DETACH => (),
        _ => ()
    }
    minwindef::TRUE
}

fn init() {
    thread::spawn(move || {
        let replacement = |internet, _, _, _| internet;
        let hook = unsafe {
            DetourConnect.initialize(WinHttpConnect, replacement).unwrap()
        };
        let hook_box = Rc::new(Mutex::new(hook));
        let hook_box_remove = hook_box.clone();
        let install = move || {
            unsafe { (*hook_box.lock().unwrap()).enable().unwrap() };
        };
        let remove = move || {
            unsafe { (*hook_box_remove.lock().unwrap()).disable().unwrap() };
        };
        server::start(install, remove).unwrap();
    });
}
