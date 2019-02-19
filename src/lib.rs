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
    ptr
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

mod server;
mod win;

type Winptr = winapi::ctypes::c_void;
type WinHttpConnectFun = unsafe extern "system" fn(HINTERNET, LPCWSTR, INTERNET_PORT, DWORD) -> HINTERNET;
type WinHttpSendRequestFun = unsafe extern "system" fn(HINTERNET, LPCWSTR, DWORD, LPVOID, DWORD, DWORD, DWORD_PTR) -> BOOL;
type WinHttpOpenRequestFun = unsafe extern "system" fn(HINTERNET, LPCWSTR, LPCWSTR, LPCWSTR, LPCWSTR, *mut LPCWSTR, DWORD) -> HINTERNET;

static_detours! {
    struct DetourConnect: unsafe extern "system" fn(HINTERNET, LPCWSTR, INTERNET_PORT, DWORD) -> HINTERNET;
    struct DetourSendRequest: unsafe extern "system" fn(HINTERNET, LPCWSTR, DWORD, LPVOID, DWORD, DWORD, DWORD_PTR) -> BOOL;
    struct DetourOpenRequest: unsafe extern "system" fn(HINTERNET, LPCWSTR, LPCWSTR, LPCWSTR, LPCWSTR, *mut LPCWSTR, DWORD) -> HINTERNET;
}

type DetourResult = Result<(), detour::Error>;

trait DetourFunction {
    unsafe fn enable(&mut self) -> DetourResult;
    unsafe fn disable(&mut self) -> DetourResult;
}

impl DetourFunction for detour::StaticDetour<WinHttpConnectFun> {
    unsafe fn enable(&mut self) -> DetourResult {
        (**self).enable()
    }
    unsafe fn disable(&mut self) -> DetourResult {
        (**self).disable()
    }
}

impl DetourFunction for detour::StaticDetour<WinHttpSendRequestFun> {
    unsafe fn enable(&mut self) -> DetourResult {
        (**self).enable()
    }
    unsafe fn disable(&mut self) -> DetourResult {
        (**self).disable()
    }
}

impl DetourFunction for detour::StaticDetour<WinHttpOpenRequestFun> {
    unsafe fn enable(&mut self) -> DetourResult {
        (**self).enable()
    }
    unsafe fn disable(&mut self) -> DetourResult {
        (**self).disable()
    }
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
        let win_http_open_request = unsafe {
            let addr = get_proc_address("winhttp.dll", "WinHttpOpenRequest");
            std::mem::transmute::<*mut winapi::shared::minwindef::__some_function,
                                  WinHttpOpenRequestFun>(addr)
        };
        let tx2 = tx.clone();
        let tx3 = tx.clone();
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
        let whor_replacement = move |connect, verb, object_name, version, referrer, accept_types, flags| {
            let verb_str = if verb == ptr::null() {
                String::from("GET")
            } else {
                let wstr = unsafe {
                    U16CString::from_ptr_str(verb)
                };
                wstr.to_string().unwrap()
            };
            let object_name_str = {
                let wstr = unsafe {
                    U16CString::from_ptr_str(object_name)
                };
                wstr.to_string().unwrap()
            };
            tx3.send(format!("{} {}", verb_str, object_name_str)).unwrap();
            unsafe {
                DetourOpenRequest.get().unwrap().call(connect, verb, object_name, version, referrer, accept_types, flags)
            }
        };
        let hooks: Vec<Box<dyn DetourFunction>> = vec!(
            Box::new(unsafe {
                DetourConnect.initialize(win_http_connect, replacement).unwrap()
            }),
            Box::new(unsafe {
                DetourSendRequest.initialize(win_http_send_request, whsr_replacement).unwrap()
            }),
            Box::new(unsafe {
                DetourOpenRequest.initialize(win_http_open_request, whor_replacement).unwrap()
            })
        );
        let hook_boxes: Vec<Rc<Mutex<Box<dyn DetourFunction>>>> = hooks.into_iter().map(|hook| Rc::new(Mutex::new(hook))).collect();
        let hook_boxes_remove: Vec<Rc<Mutex<Box<dyn DetourFunction>>>> = hook_boxes.iter().map(|hbox| hbox.clone()).collect();
        let install = move || {
            hook_boxes.iter().for_each(|hbox| unsafe {
                hbox.lock().unwrap().enable().unwrap();
            })
        };
        let remove = move || {
            hook_boxes_remove.iter().for_each(|hbox| unsafe {
                hbox.lock().unwrap().disable().unwrap();
            })
        };
        let looper = move || {
            rx.try_iter().collect::<Vec<String>>()
        };
        server::start(install, looper, remove).unwrap();
    });
}
