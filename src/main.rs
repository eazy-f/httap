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
    PROCESS_VM_WRITE,
    MEM_COMMIT,
    PAGE_EXECUTE_READWRITE
};
#[cfg(windows)]
use winapi::um::libloaderapi::{GetModuleHandleA, GetProcAddress};
#[cfg(windows)]
use winapi::shared::ntdef::FALSE;
#[cfg(windows)]
use winapi::um::memoryapi::{VirtualAllocEx, WriteProcessMemory};
#[cfg(windows)]
use winapi::um::errhandlingapi::GetLastError;
#[cfg(windows)]
use std::ffi::CString;

#[cfg(windows)]
use std::ptr;

#[cfg(not(windows))]
mod server;

#[cfg(windows)]
type Winptr = winapi::ctypes::c_void;

#[cfg(windows)]
fn win_panic_handle(handle: *const winapi::ctypes::c_void, err: &'static str) {
    if handle == ptr::null() {
        winpanic(err);
    }
}

#[cfg(windows)]
fn win_panic_code(code: i32, err: &'static str) {
    if code == 0 {
        winpanic(err);
    }
}

#[cfg(windows)]
fn winpanic(err: &'static str) {
    let ec = unsafe { GetLastError() };
    println!("error code: {}", ec);
    panic!(err);
}

#[cfg(windows)]
fn load(pid: u32, dll: &String){
    let process = unsafe {
        let access = PROCESS_CREATE_THREAD | PROCESS_VM_OPERATION | PROCESS_VM_WRITE;
        OpenProcess(access, FALSE as i32, pid)
    };
    win_panic_handle(process, "cannot open process");
    let kernel32_file = CString::new("kernel32.dll").unwrap();
    let inject_dll = CString::new(dll.as_bytes()).unwrap();
    let alloc_size = inject_dll.as_bytes_with_nul().len();
    let allocation = unsafe {
        VirtualAllocEx(process, ptr::null_mut(), alloc_size, MEM_COMMIT, PAGE_EXECUTE_READWRITE)
    };
    win_panic_handle(allocation, "cannot allocate memory in the remote process");
    let write_res = unsafe {
        WriteProcessMemory(
            process, allocation, inject_dll.as_ptr() as *mut Winptr,
            alloc_size, ptr::null_mut()
        )
    };
    win_panic_code(write_res, "cannot write to process memory");
    let stack_size = 512 * 1024;
    let creation = 0;
    let security = ptr::null_mut();
    let thread_id = ptr::null_mut();
    let start = unsafe {
        let ptr = std::mem::transmute::<*mut winapi::ctypes::c_void, unsafe extern "system" fn(*mut winapi::ctypes::c_void) -> u32>(allocation);
        Some(ptr)
    };
    let loadlibrary_fun = CString::new("LoadLibraryA").unwrap();
    let kernel32_module = unsafe { GetModuleHandleA(kernel32_file.as_ptr()) };
    win_panic_handle(kernel32_module as *const Winptr, "cannot get kernel32 module handle");
    let loadlibrary_addr = unsafe { GetProcAddress(kernel32_module, loadlibrary_fun.as_ptr()) };
    win_panic_handle(loadlibrary_addr as *const Winptr, "cannot find LoadLibraryA address");
    let loadlibrary_ptr = unsafe {
        let ptr = std::mem::transmute::<*mut winapi::shared::minwindef::__some_function, unsafe extern "system" fn(*mut winapi::ctypes::c_void) -> u32>(loadlibrary_addr);
        Some(ptr)
    };
    let thread = unsafe {
        CreateRemoteThread(
            process, security, stack_size, loadlibrary_ptr,
            allocation, creation, thread_id
        )
    };
    win_panic_handle(thread, "failed to create remote thread");
    println!("thread id: {}", thread as u64);
}

#[cfg(not(windows))]
fn load(_pid: u32, _dll: &String){
    use std::thread;
    let server = thread::spawn(move || { server::start().unwrap(); });
    server.join().unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let pid = &args[1];
    let dll = &args[2];
    load(pid.parse::<u32>().unwrap(), dll);
}
