extern crate serde;

use kclvm_runner::eval::Evaluator;
use kclvm_runner::runner::*;

#[no_mangle]
pub extern "C" fn kclvm_cli_run(args: *const i8, plugin_agent: *const i8) -> *const i8 {
    let args = ExecProgramArgs::from_str(kclvm::c2str(args));
    let plugin_agent = plugin_agent as u64;

    let files = args.get_files();
    let opts = args.get_load_program_options();

    // link all dylibs
    let dylib_path = Evaluator::new().eval_files_with_opts(files, opts, &args, plugin_agent);

    // Config uild
    // run dylib
    let runner = KclvmRunner::new(
        dylib_path.as_str(),
        Some(KclvmRunnerOptions {
            plugin_agent_ptr: plugin_agent,
        }),
    );

    match runner.run(&args) {
        Ok(result) => {
            let c_string = std::ffi::CString::new(result.as_str()).expect("CString::new failed");
            let ptr = c_string.into_raw();
            ptr as *const i8
        }
        Err(result) => {
            let result = format!("ERROR:{}", result);
            let c_string = std::ffi::CString::new(result.as_str()).expect("CString::new failed");
            let ptr = c_string.into_raw();
            ptr as *const i8
        }
    }
}
