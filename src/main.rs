extern crate winapi;

#[cfg(windows)]
use winapi::{
    um::{
        libloaderapi::{GetModuleHandleA, GetProcAddress},
        memoryapi::{VirtualAllocEx, WriteProcessMemory},
        errhandlingapi::GetLastError,
        winnt::{
            PROCESS_CREATE_THREAD,
            PROCESS_VM_OPERATION,
            PROCESS_VM_WRITE,
            MEM_COMMIT,
            PAGE_EXECUTE_READWRITE,
            TOKEN_ALL_ACCESS,
            HANDLE,
            LUID,
            TOKEN_PRIVILEGES,
            LUID_AND_ATTRIBUTES,
            SE_PRIVILEGE_ENABLED
        },
        processthreadsapi::{
            OpenProcess,
            CreateRemoteThread,
            OpenProcessToken,
            GetCurrentProcess
        },
        securitybaseapi::AdjustTokenPrivileges,
        winbase::LookupPrivilegeValueA
    },
    shared::ntdef::FALSE
};

#[cfg(windows)]
use std::ffi::CString;

#[cfg(windows)]
use std::{ptr, mem};

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
fn enable_debug() {
    let process = unsafe { GetCurrentProcess() };
    let token = unsafe {
        let mut token: HANDLE = mem::uninitialized();
        let success = OpenProcessToken(process, TOKEN_ALL_ACCESS, &mut token);
        win_panic_code(success, "cannot open current process access token");
        token
    };
    let privilege = unsafe {
        let priv_name = CString::new("SeDebugPrivilege").unwrap();
        let mut uuid: LUID = mem::uninitialized();
        let success = LookupPrivilegeValueA(ptr::null_mut(), priv_name.as_ptr(), &mut uuid);
        win_panic_code(success,"cannot lookup debug privilege");
        uuid
    };
    let mut privileges = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {Luid: privilege, Attributes: SE_PRIVILEGE_ENABLED}]
    };
    let adjusted = unsafe {
        AdjustTokenPrivileges(
            token,
            FALSE as i32,
            &mut privileges,
            mem::size_of::<TOKEN_PRIVILEGES>() as u32,
            ptr::null_mut(),
            ptr::null_mut()
        )
    };
    win_panic_code(adjusted, "cannot adjust token privileges");
}

#[cfg(windows)]
fn load(pid: u32, dll: &String){
    enable_debug();
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
    use std::{thread, sync, time, rc};

    let server = thread::spawn(move || {
        let start_time_a = rc::Rc::new(sync::Mutex::new(None));
        let start_time_b = start_time_a.clone();
        let start = move || {
            *start_time_a.lock().unwrap() = Some(time::SystemTime::now());
        };
        let end = move || {
            let mut time = start_time_b.lock().unwrap();
            println!("{}", (*time).unwrap().duration_since(time::SystemTime::UNIX_EPOCH).unwrap().as_secs());
            *time = None;
        };
        server::start(start, || 0, end).unwrap();
    });
    server.join().unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let pid = &args[1];
    let dll = &args[2];
    load(pid.parse::<u32>().unwrap(), dll);
}
