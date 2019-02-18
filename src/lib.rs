#![cfg(windows)]
extern crate winapi;
#[macro_use]
extern crate detour;
extern crate widestring;

use std::{
    thread,
    sync::{Mutex, mpsc},
    rc::Rc,
    ffi::CString,
    ops::Deref
};

use winapi::{
    um::{
        winnt::LPCWSTR,
        winhttp::{HINTERNET, INTERNET_PORT},
        libloaderapi::{GetModuleHandleA, GetProcAddress}
    },
    shared::{
        minwindef,
        minwindef::{BOOL, DWORD, HINSTANCE, LPVOID},
        basetsd::DWORD_PTR
    }
};

use widestring::U16CString;
use detour::{Function, GenericDetour};

mod server;
mod win;

type Winptr = winapi::ctypes::c_void;
type WinHttpConnectFun = unsafe extern "system" fn(HINTERNET, LPCWSTR, INTERNET_PORT, DWORD) -> HINTERNET;
type WinHttpSendRequestFun = unsafe extern "system" fn(HINTERNET, LPCWSTR, DWORD, LPVOID, DWORD, DWORD, DWORD_PTR) -> BOOL;

static_detours! {
    struct DetourConnect: unsafe extern "system" fn(HINTERNET, LPCWSTR, INTERNET_PORT, DWORD) -> HINTERNET;
    struct DetourSendRequest: unsafe extern "system" fn(HINTERNET, LPCWSTR, DWORD, LPVOID, DWORD, DWORD, DWORD_PTR) -> BOOL;
}

trait DetourFunction<T: Function>: Deref<Target=GenericDetour<T>> {}

impl DetourFunction<WinHttpConnectFun> for detour::StaticDetour<WinHttpConnectFun> {}
impl DetourFunction<WinHttpSendRequestFun> for detour::StaticDetour<WinHttpSendRequestFun> {}

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

fn get_proc_address(module: &str, function: &str) -> *mut winapi::shared::minwindef::__some_function {
    let module_name = CString::new(module).unwrap();
    let fun_name = CString::new(function).unwrap();
    let module = unsafe { GetModuleHandleA(module_name.as_ptr()) };
    win::panic_handle(module as *const Winptr, "cannot get module handle");
    unsafe {
        let addr = GetProcAddress(module, fun_name.as_ptr());
        win::panic_handle(addr as *const Winptr, "cannot get function address");
        addr
    }
}

fn init() {
    thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let win_http_connect = unsafe {
            let addr = get_proc_address("winhttp.dll", "WinHttpConnect");
            std::mem::transmute::<*mut winapi::shared::minwindef::__some_function, WinHttpConnectFun>(addr)
        };
        let win_http_send_request = unsafe {
            let addr = get_proc_address("winhttp.dll", "WinHttpSendRequest");
            std::mem::transmute::<*mut winapi::shared::minwindef::__some_function,
                                  WinHttpSendRequestFun>(addr)
        };
        let tx2 = tx.clone();
        let replacement = move |session, server, port, reserved| {
            let server_str = unsafe {
                U16CString::from_ptr_str(server)
            };
            tx.send(server_str.to_string().unwrap()).unwrap();
            unsafe {
                DetourConnect.get().unwrap().call(session, server, port, reserved)
            }
        };
        let whsr_replacement = move |request, headers, headers_len, optional, optional_length, total_length, context| {
            let headers_str = unsafe {
                U16CString::from_ptr_str(headers)
            };
            tx2.send(headers_str.to_string().unwrap()).unwrap();
            unsafe {
                DetourSendRequest.get().unwrap().call(request, headers, headers_len, optional, optional_length, total_length, context)
            }
        };
        let hooks: [Box<dyn DetourFunction>] = [
            Box::new(unsafe {
                DetourConnect.initialize(win_http_connect, replacement).unwrap()
            }),
            Box::new(unsafe {
                DetourSendRequest.initialize(win_http_send_request, whsr_replacement).unwrap()
            })
        ];
/*        let hook_boxes = hooks.into_iter().map(|hook| Rc::new(Mutex::new(*hook))).collect();
        let hook_boxes_remove = hook_boxes.iter().map(|hbox| hbox.clone()).collect();*/
        let install = move || {
//            hook_boxes.into_iter().for_each(|hbox| unsafe {
//                (*hbox.lock().unwrap()).enable().unwrap();
//            })
        };
        let remove = move || {
//            hook_boxes_remove.into_iter().for_each(|hbox| unsafe {
//                (*hbox.lock().unwrap()).disable().unwrap();
//            })
        };
        let looper = move || {
            rx.try_iter().collect::<Vec<String>>()
        };
        server::start(install, looper, remove).unwrap();
    });
}
