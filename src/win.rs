#![cfg(windows)]
use winapi::um::errhandlingapi::GetLastError;

use std::ptr;

pub fn panic_handle(handle: *const winapi::ctypes::c_void, err: &'static str) {
    if handle == ptr::null() {
        winpanic(err);
    }
}

pub fn panic_code(code: i32, err: &'static str) {
    if code == 0 {
        winpanic(err);
    }
}

fn winpanic(err: &'static str) {
    let ec = unsafe { GetLastError() };
    println!("error code: {}", ec);
    panic!(err);
}

