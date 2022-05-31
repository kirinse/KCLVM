mod wasm;

use once_cell::sync::Lazy;
use std::ffi::CString;
use std::sync::Mutex;

static LINKER_MUTEX: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(0i32));

/// The compile target.
#[derive(PartialEq, Clone, Copy)]
pub enum Target {
    /// Generate a generic object file for linking.
    Generic,
    /// Generate a WASM module file for linking.
    Wasm,
}

impl Target {
    /// File extension
    pub fn file_extension(&self) -> &'static str {
        match self {
            Target::Generic => "so",
            Target::Wasm => "wasm",
        }
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Target::Generic => write!(f, "generic"),
            Target::Wasm => write!(f, "wasm"),
        }
    }
}

/// Take an object file and turn it into a final linked binary ready for deployment
pub fn link(input: &[u8], name: &str, export_names: &[String], target: Target) -> Vec<u8> {
    // The lld linker is totally not thread-safe; it uses many globals
    // We should fix this one day
    let _lock = LINKER_MUTEX.lock().unwrap();
    match &target {
        Target::Wasm => wasm::link(input, name, export_names),
        Target::Generic => todo!("generic target link"),
    }
}

#[link(name = "linker")]
extern "C" {
    //fn lldMain_gnu(args: *const *const libc::c_char, size: libc::size_t) -> libc::c_int;
    //fn lldMain_gnu_pe(args: *const *const libc::c_char, size: libc::size_t) -> libc::c_int;
    //fn lldMain_darwin(args: *const *const libc::c_char, size: libc::size_t) -> libc::c_int;

    //fn lldMain_darwin_new(argc: libc::c_int, args: *const *const libc::c_char) -> libc::c_int;
    pub fn lldMain_wasm      (argc: libc::c_int, args: *const *const libc::c_char) -> libc::c_int;
}


pub fn clang_main_foo() {
    println!("clang_main_foo");

    let args = vec![CString::new("hello.c").unwrap()];

    clang_main(&args);
}

pub fn clang_main(args: &[CString]) -> bool {
    let mut command_line: Vec<*const libc::c_char> = Vec::with_capacity(args.len() + 1);
    let executable_name = CString::new("clang").unwrap();

    command_line.push(executable_name.as_ptr());

    for arg in args {
        command_line.push(arg.as_ptr());
    }

    //unsafe { lldMain_darwin_new(command_line.len() as i32, command_line.as_ptr()) == 0 }
    unsafe { lldMain_wasm(command_line.len() as i32, command_line.as_ptr()) == 0 }
  // false
}

pub fn wasm_linker(args: &[CString]) -> bool {
    let mut command_line: Vec<*const libc::c_char> = Vec::with_capacity(args.len() + 1);

    let executable_name = CString::new("wasm-ld").unwrap();

    command_line.push(executable_name.as_ptr());

    for arg in args {
        command_line.push(arg.as_ptr());
    }

    unsafe { lldMain_wasm(command_line.len() as i32, command_line.as_ptr()) == 0 }
}
