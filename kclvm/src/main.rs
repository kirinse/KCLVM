//! The `kclvm` command-line interface.

#[macro_use]
extern crate clap;

use clap::ArgMatches;
use kclvm_config::settings::{load_file, merge_settings, SettingsFile};
use kclvm_parser::parse_file;
use kclvm_runner::eval::Evaluator;

fn main() {
    let matches = clap_app!(kcl =>
        (@subcommand run =>
            (@arg INPUT: ... "Sets the input file to use")
            (@arg OUTPUT: -o --output +takes_value "Sets the LLVM IR/BC output file path")
            (@arg SETTING: ... -Y --setting "Sets the input file to use")
            (@arg EMIT_TYPE: --emit +takes_value "Sets the emit type, expect (ast)")
            (@arg BC_PATH: --bc +takes_value "Sets the linked LLVM bitcode file path")
            (@arg verbose: -v --verbose "Print test information verbosely")
            (@arg disable_none: -n --disable-none "Disable dumping None values")
            (@arg debug: -d --debug "Run in debug mode (for developers only)")
            (@arg sort_key: -k --sort "Sort result keys")
            (@arg ARGUMENT: ... -D --argument "Specify the top-level argument")
        )
    )
    .get_matches();
    if let Some(matches) = matches.subcommand_matches("run") {
        if let Some(files) = matches.values_of("INPUT") {
            let files: Vec<&str> = files.into_iter().collect::<Vec<&str>>();
            if let Some(emit_ty) = matches.value_of("EMIT_TYPE") {
                if emit_ty == "ast" {
                    let module = parse_file(files[0], None);
                    println!("{}", serde_json::to_string(&module).unwrap())
                }
            } else {
                let settings = build_settings(&matches);
                Evaluator::new().eval_files_with_settings(files, settings, 0);
            }
        } else {
            println!("{}", matches.usage());
        }
    } else {
        println!("{}", matches.usage());
    }
}

/// Build settings from arg matches.
fn build_settings(matches: &ArgMatches) -> SettingsFile {
    let debug_mode = matches.occurrences_of("debug") > 0;
    let disable_none = matches.occurrences_of("disable_none") > 0;

    let mut settings = if let Some(files) = matches.values_of("SETTING") {
        let files: Vec<&str> = files.into_iter().collect::<Vec<&str>>();
        merge_settings(
            &files
                .iter()
                .map(|f| load_file(f))
                .collect::<Vec<SettingsFile>>(),
        )
    } else {
        SettingsFile::new()
    };
    if let Some(config) = &mut settings.kcl_cli_configs {
        config.debug = Some(debug_mode);
        config.disable_none = Some(disable_none);
    }
    settings
}
