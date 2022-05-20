use std::{collections::HashMap, path::Path};
use indexmap::IndexSet;

use kclvm_ast::ast::*;
use kclvm_error::Diagnostic;
use kclvm_config::settings::SettingsFile;
use kclvm_parser::{load_program, LoadProgramOptions};
use kclvm_tools::query::apply_overrides;
use crate::{command::Command, libgen::DyLibGenerator, runner::{ExecProgramArgs, KclvmRunner, KclvmRunnerOptions}};

const MAIN_PKG_NAME: &str = "__main__";
pub struct Evaluator;

impl Evaluator{
    pub fn new() -> Self{
        Self{}
    }

    pub fn eval_files_with_settings(&self, files: Vec<&str>, settings: SettingsFile, plugin_agent: u64){
        let program = load_program(&files, None);
        self.build_and_run_program_with_settings(program, settings, plugin_agent);
    }

    pub fn eval_files_with_opts(&self, files: Vec<&str>, opts: LoadProgramOptions, args: &ExecProgramArgs, plugin_agent: u64) -> String{
        let mut program = load_program(&files, Some(opts));
        apply_overrides(&mut program, &args.overrides, &[]);
        self.build_and_run_program(program, plugin_agent)
    }

    pub fn build_and_run_program(&self, program: Program, plugin_agent: u64) -> String{
        let mut dylib_paths = Vec::new();
        let dylib_run_result= DyLibGenerator::gen_and_run_dylib_from_ast(program, plugin_agent);
        match dylib_run_result {
            Ok(paths) => dylib_paths = paths,
            // todo @shijun.zong: how to deal the errs
            _ => {}
        }
        let mut cmd = Command::new(plugin_agent);
        // link all dylibs
        let dylib_path = cmd.link_dylibs(&dylib_paths, "");
        // Config build
        // let settings = build_settings(&matches);
        dylib_path
    }

    pub fn eval_code(&self, file_name: &str, code: Option<String>, plugin_agent: u64){
        let module = kclvm_parser::parse_file(file_name, code);
        self.eval_main_module(module, plugin_agent);
    }

    pub fn eval_main_module(&self, mut module: Module, plugin_agent: u64) -> IndexSet<Diagnostic>{
        // module name changed here
        module.pkg = MAIN_PKG_NAME.to_string();
        self.build_and_run_module_with_settings(module, build_default_settings(), plugin_agent)
    }

    pub fn build_and_run_module_with_settings(&self, module: Module, settings: SettingsFile, plugin_agent: u64) -> IndexSet<Diagnostic>{
        let mut pkgs_ast = HashMap::new();
        pkgs_ast.insert(MAIN_PKG_NAME.to_string(), vec![module]);
        // load ast
        let program = Program{
            root: MAIN_PKG_NAME.to_string(),
            main: MAIN_PKG_NAME.to_string(),
            pkgs: pkgs_ast,
            cmd_args: vec![],
            cmd_overrides: vec![]
        };

        let run_result = DyLibGenerator::gen_and_run_dylib_from_ast(program, plugin_agent);
        let mut dylib_paths = Vec::new();
        let diagnostics = IndexSet::new();
        match run_result{
            Ok(mut paths) => dylib_paths.append(&mut paths),
            Err(dias) => {
                return dias;
            }
        }
        let mut cmd = Command::new(0);
        // link all dylibs
        let dylib_path = cmd.link_dylibs(&dylib_paths, "");
        cmd.run_dylib_with_settings(&dylib_path, settings).unwrap();
        // cmd.run_dylib(&dylib_path);
        for dylib_path in dylib_paths {
            if dylib_path.contains(kclvm_ast::MAIN_PKG) && Path::new(&dylib_path).exists() {
                std::fs::remove_file(&dylib_path).unwrap();
            }
        }
        diagnostics
    }

    pub fn build_and_run_program_with_settings(&self, program: Program, settings: SettingsFile, plugin_agent: u64){
        let mut dylib_paths = Vec::new();
        let dylib_run_result= DyLibGenerator::gen_and_run_dylib_from_ast(program, plugin_agent);
        match dylib_run_result {
            Ok(paths) => dylib_paths = paths,
            // todo @shijun.zong: how to deal the errs
            _ => {}
        }

        let mut cmd = Command::new(0);
        // link all dylibs
        let dylib_path = cmd.link_dylibs(&dylib_paths, "");
        // Config build
        // let settings = build_settings(&matches);
        cmd.run_dylib_with_settings(&dylib_path, settings).unwrap();
        for dylib_path in dylib_paths {
            if dylib_path.contains(kclvm_ast::MAIN_PKG) && Path::new(&dylib_path).exists() {
                std::fs::remove_file(&dylib_path).unwrap();
            }
        }
    }
}

fn build_default_settings() -> SettingsFile {
    let debug_mode = false;
    let disable_none = false;

    let mut settings = SettingsFile::new();

    if let Some(config) = &mut settings.kcl_cli_configs {
        config.debug = Some(debug_mode);
        config.disable_none = Some(disable_none);
    }
    settings
}
