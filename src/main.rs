extern crate winapi;

#[cfg(windows)]
use winapi::um::processthreadsapi::OpenProcess;
#[cfg(windows)]
use winapi::um::winnt::PROCESS_CREATE_THREAD;
#[cfg(windows)]
use winapi::shared::ntdef::FALSE;

#[cfg(windows)]
fn load(){
    unsafe {
        OpenProcess(PROCESS_CREATE_THREAD, FALSE as i32, 0x1);
    }
}

#[cfg(not(windows))]
fn load(){
    println!("Hello, world!");
}


fn main() {
    load();
}
