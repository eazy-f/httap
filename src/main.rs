extern crate winapi;

#[cfg(windows)]
use winapi::um::processthreadsapi::OpenProcess;
#[cfg(windows)]
use winapi::um::winnt::{
    PROCESS_CREATE_THREAD,
    PROCESS_VM_OPERATION,
    MEM_COMMIT,
    PAGE_EXECUTE_READWRITE
};
#[cfg(windows)]
use winapi::shared::ntdef::FALSE;
#[cfg(windows)]
use winapi::um::memoryapi::VirtualAllocEx;
#[cfg(windows)]
use winapi::um::errhandlingapi::GetLastError;

use std::ptr;
use std::env;

#[cfg(windows)]
fn win_panic_handle(handle: *const winapi::ctypes::c_void, err: &'static str) {
    if handle == ptr::null() {
        let ec = unsafe { GetLastError() };
        println!("error code: {}", ec);
        panic!(err);
    }
}

#[cfg(windows)]
fn load(pid: u32){
    let process = unsafe {
        OpenProcess(PROCESS_CREATE_THREAD | PROCESS_VM_OPERATION, FALSE as i32, pid)
    };
    win_panic_handle(process, "Cannot open process");
    let alloc_size = 1024*1024;
    let allocation = unsafe {
        VirtualAllocEx(process, ptr::null_mut(), alloc_size, MEM_COMMIT, PAGE_EXECUTE_READWRITE)
    };
    win_panic_handle(allocation, "Cannot allocate memory in the remote process");
}

#[cfg(not(windows))]
fn load(pid: u32){
    println!("Hello, world!");
}


fn main() {
    let pid = std::env::args().nth(1).unwrap();
    load(pid.parse::<u32>().unwrap());
}
