#![cfg(windows)]
extern crate winapi;
#[macro_use]
extern crate detour;

use std::{
    thread,
    sync::{Mutex, mpsc},
    rc::Rc,
    ffi::CString
};

use winapi::{
    um::{
        winnt::LPCWSTR,
        winhttp::{HINTERNET, INTERNET_PORT, WinHttpConnect},
        libloaderapi::{GetModuleHandleA, GetProcAddress}
    },
    shared::{
        minwindef,
        minwindef::{BOOL, DWORD, HINSTANCE, LPVOID},
    }
};

mod server;
mod win;

type Winptr = winapi::ctypes::c_void;
type WinHttpConnectFun = unsafe extern "system" fn(HINTERNET, LPCWSTR, INTERNET_PORT, DWORD) -> HINTERNET;

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
        let (tx, rx) = mpsc::channel();
        let win_http_lib = CString::new("winhttp.dll").unwrap();
        let win_http_connect_fun = CString::new("WinHttpConnect").unwrap();
        let win_http_module = unsafe { GetModuleHandleA(win_http_lib.as_ptr()) };
        win::panic_handle(win_http_module as *const Winptr, "cannot get winhttp module handle");
        let win_http_connect = unsafe {
            let addr = GetProcAddress(win_http_module, win_http_connect_fun.as_ptr());
            win::panic_handle(addr as *const Winptr, "cannot get WinHttpConnect address");
            std::mem::transmute::<*mut winapi::shared::minwindef::__some_function, WinHttpConnectFun>(addr)
        };
        let replacement = move |session, server, port, reserved| {
            tx.send(0).unwrap();
            unsafe {
                DetourConnect.get().unwrap().call(session, server, port, reserved)
            }
        };
        let hook = unsafe {
            DetourConnect.initialize(win_http_connect, replacement).unwrap()
        };
        let hook_box = Rc::new(Mutex::new(hook));
        let hook_box_remove = hook_box.clone();
        let install = move || {
            unsafe { (*hook_box.lock().unwrap()).enable().unwrap() };
        };
        let remove = move || {
            unsafe { (*hook_box_remove.lock().unwrap()).disable().unwrap() };
        };
        let looper = move || {
            rx.try_iter().count()
        };
        server::start(install, looper, remove).unwrap();
    });
}
