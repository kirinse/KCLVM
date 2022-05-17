use std::{collections::HashMap, path::Path};

use kclvm_ast::ast::*;

use crate::{command::Command, libgen::DyLibGenerator};

const MAIN_PKG_NAME: &str = "__main__";
pub struct Evaluator;

impl Evaluator{
    pub fn eval_code(&self, file_name: &str, code: Option<String>){
        let module = kclvm_parser::parse_file(file_name, code);
        self.eval_main_module(module);
    }

    pub fn eval_main_module(&self, mut module: Module){
        // 这里需要做防御或者注释通知，该懂了module
        // 因为这个改名字属于代码的副作用。
        module.pkg = MAIN_PKG_NAME.to_string();
        self.build_and_run_ast(module);
    }

    pub fn build_and_run_ast(&self, module: Module){
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
        let dylib_paths = DyLibGenerator::gen_and_run_dylib_from_ast(program);
        let mut cmd = Command::new(0);
       
        // link all dylibs
        let dylib_path = cmd.link_dylibs(&dylib_paths, "");
        cmd.run_dylib(&dylib_path);
        for dylib_path in dylib_paths {
            if dylib_path.contains(kclvm_ast::MAIN_PKG) && Path::new(&dylib_path).exists() {
                std::fs::remove_file(&dylib_path).unwrap();
            }
        }
    }
}
