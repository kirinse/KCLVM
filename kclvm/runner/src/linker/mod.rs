use std::ffi::CString;
use once_cell::sync::Lazy;
use std::sync::Mutex;

static LINKER_MUTEX: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(0i32));

extern "C" {
    fn LLDWasmLink(args: *const *const libc::c_char, size: libc::size_t) -> libc::c_int;
    fn LLDDarwinLink(args: *const *const libc::c_char, size: libc::size_t) -> libc::c_int;
    fn LLDDarwinNewLink(args: *const *const libc::c_char, size: libc::size_t) -> libc::c_int;
    fn LLDGnuNewLink(args: *const *const libc::c_char, size: libc::size_t) -> libc::c_int;
    fn LLDMingwLink(args: *const *const libc::c_char, size: libc::size_t) -> libc::c_int;
}

pub fn wasm_linker(args: &[CString]) -> bool {
    let mut command_line: Vec<*const libc::c_char> = Vec::with_capacity(args.len() + 1);

    let executable_name = CString::new("wasm-ld").unwrap();

    command_line.push(executable_name.as_ptr());

    for arg in args {
        command_line.push(arg.as_ptr());
    }

    unsafe { LLDWasmLink(command_line.as_ptr(), command_line.len()) == 0 }
}

pub fn darwin_linker(args: &[CString]) -> bool {
    let _lock = LINKER_MUTEX.lock().unwrap();
    let mut command_line: Vec<*const libc::c_char> = Vec::with_capacity(args.len() + 1);

    let executable_name = CString::new("lld").unwrap();

    command_line.push(executable_name.as_ptr());

    for arg in args {
        command_line.push(arg.as_ptr());
    }

    unsafe { LLDDarwinLink(command_line.as_ptr(), command_line.len()) == 0 }
}

pub fn darwin_new_linker(args: &[CString]) -> bool {
    let mut command_line: Vec<*const libc::c_char> = Vec::with_capacity(args.len() + 1);

    let executable_name = CString::new("wasm-ld").unwrap();

    command_line.push(executable_name.as_ptr());

    for arg in args {
        command_line.push(arg.as_ptr());
    }

    unsafe { LLDDarwinNewLink(command_line.as_ptr(), command_line.len()) == 0 }
}

pub fn linux_linker(args: &[CString]) -> bool {
    let mut command_line: Vec<*const libc::c_char> = Vec::with_capacity(args.len() + 1);

    let executable_name = CString::new("lld").unwrap();

    command_line.push(executable_name.as_ptr());

    for arg in args {
        command_line.push(arg.as_ptr());
    }

    unsafe { LLDGnuNewLink(command_line.as_ptr(), command_line.len()) == 0 }
}

pub fn mingw_linker(args: &[CString]) -> bool {
    let mut command_line: Vec<*const libc::c_char> = Vec::with_capacity(args.len() + 1);

    let executable_name = CString::new("lld").unwrap();

    command_line.push(executable_name.as_ptr());

    for arg in args {
        command_line.push(arg.as_ptr());
    }

    unsafe { LLDMingwLink(command_line.as_ptr(), command_line.len()) == 0 }
}
