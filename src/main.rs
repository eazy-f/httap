extern crate winapi;

#[cfg(windows)]
use winapi::um::processthreadsapi::{
    OpenProcess,
    CreateRemoteThread
};
#[cfg(windows)]
use winapi::um::winnt::{
    PROCESS_CREATE_THREAD,
    PROCESS_VM_OPERATION,
    MEM_COMMIT,
    PAGE_EXECUTE_READWRITE
};
#[cfg(windows)]
use winapi::um::winbase::CREATE_SUSPENDED;
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
    win_panic_handle(process, "cannot open process");
    let alloc_size = 1024*1024;
    let allocation = unsafe {
        VirtualAllocEx(process, ptr::null_mut(), alloc_size, MEM_COMMIT, PAGE_EXECUTE_READWRITE)
    };
    win_panic_handle(allocation, "cannot allocate memory in the remote process");
    let stack_size = 512 * 1024;
    let parameter = ptr::null_mut();
    let creation = CREATE_SUSPENDED;
    let security = ptr::null_mut();
    let thread_id = ptr::null_mut();
    let start = unsafe {
        let ptr = std::mem::transmute::<*mut winapi::ctypes::c_void, unsafe extern "system" fn(*mut winapi::ctypes::c_void) -> u32>(allocation);
        Some(ptr)
    };
    let thread = unsafe {
        CreateRemoteThread(
            process, security, stack_size, start,
            parameter, creation, thread_id
        )
    };
    win_panic_handle(thread, "failed to create remote thread");
    println!("thread id: {}", thread as u64);
}

#[cfg(not(windows))]
fn load(pid: u32){
    println!("Hello, world!");
}


fn main() {
    let pid = std::env::args().nth(1).unwrap();
    load(pid.parse::<u32>().unwrap());
}
