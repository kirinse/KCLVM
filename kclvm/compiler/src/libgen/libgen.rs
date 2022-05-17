use indexmap::IndexMap;
use kclvm_config::cache::{CacheOption, load_pkg_cache, save_pkg_cache};
use std::path::PathBuf;
use std::thread;
use std::{collections::HashMap, path::Path};
use kclvm_ast::ast::*;
use kclvm_runner::command::Command;
use kclvm_sema::resolver::resolve_program;
use crate::codegen::EmitOptions;
use crate::codegen::llvm::emit_code;

const LL_FILE: &str = "_a.out";

pub struct DyLibGenerator;

impl DyLibGenerator{
    pub fn gen_dylib_from_ast(mut program: Program) -> Vec<String>{
        let scope = resolve_program(&mut program);
        let path = std::path::Path::new(LL_FILE);
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }
        for entry in glob::glob(&format!("{}*.ll", LL_FILE)).unwrap() {
            match entry {
                Ok(path) => {
                    if path.exists() {
                        std::fs::remove_file(path).unwrap();
                    }
                }
                Err(e) => println!("{:?}", e),
            };
        }

        let cache_dir = Path::new(&program.root)
        .join(".kclvm")
        .join("cache")
        .join(kclvm_version::get_full_version());

        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir).unwrap();
        }

        let mut compile_progs: IndexMap<
            String,
            (
                Program,
                IndexMap<String, IndexMap<String, String>>,
                PathBuf,
            ),
        > = IndexMap::default();

        for (pkgpath, modules) in program.pkgs {
            let mut pkgs = HashMap::new();
            pkgs.insert(pkgpath.clone(), modules);
            let compile_prog = Program {
                root: program.root.clone(),
                main: program.main.clone(),
                pkgs,
                cmd_args: vec![],
                cmd_overrides: vec![],
            };
            compile_progs.insert(
                pkgpath,
                (compile_prog, scope.import_names.clone(), cache_dir.clone()),
            );
        }

        let mut theads = vec![];
        for (pkgpath, (compile_prog, import_names, cache_dir)) in compile_progs {
            let t = thread::spawn(move || {
                let root = &compile_prog.root;
                let is_main_pkg = pkgpath == kclvm_ast::MAIN_PKG;
                let file = if is_main_pkg {
                    let main_file =
                        format!("{}{}", pkgpath, chrono::Local::now().timestamp_nanos());
                    cache_dir.join(&main_file)
                } else {
                    cache_dir.join(&pkgpath)
                };
                let lock_file =
                    format!("{}.lock", cache_dir.join(&pkgpath).to_str().unwrap());
                let ll_file = file.to_str().unwrap();
                let ll_path = format!("{}.ll", ll_file);
                let dylib_path = format!("{}{}", ll_file, Command::get_lib_suffix());
                let mut ll_path_lock = fslock::LockFile::open(&lock_file).unwrap();
                ll_path_lock.lock().unwrap();
                if Path::new(&ll_path).exists() {
                    std::fs::remove_file(&ll_path).unwrap();
                }
                let dylib_path = if is_main_pkg {
                    emit_code(
                        &compile_prog,
                        import_names,
                        &EmitOptions {
                            from_path: None,
                            emit_path: Some(&ll_file),
                            no_link: true,
                        },
                    )
                    .expect("Compile KCL to LLVM error");
                    let mut cmd = Command::new(0);
                    cmd.run_clang_single(&ll_path, &dylib_path)
                } else {
                    // If AST module has been modified, ignore the dylib cache
                    let dylib_relative_path: Option<String> =
                        load_pkg_cache(root, &pkgpath, CacheOption::default());
                    match dylib_relative_path {
                        Some(dylib_relative_path) => {
                            if dylib_relative_path.starts_with('.') {
                                dylib_relative_path.replacen(".", root, 1)
                            } else {
                                dylib_relative_path
                            }
                        }
                        None => {
                            emit_code(
                                &compile_prog,
                                import_names,
                                &EmitOptions {
                                    from_path: None,
                                    emit_path: Some(&ll_file),
                                    no_link: true,
                                },
                            )
                            .expect("Compile KCL to LLVM error");
                            let mut cmd = Command::new(0);
                            let dylib_path = cmd.run_clang_single(&ll_path, &dylib_path);
                            let dylib_relative_path = dylib_path.replacen(root, ".", 1);

                            save_pkg_cache(
                                root,
                                &pkgpath,
                                dylib_relative_path,
                                CacheOption::default(),
                            );
                            dylib_path
                        }
                    }
                };
                if Path::new(&ll_path).exists() {
                    std::fs::remove_file(&ll_path).unwrap();
                }
                ll_path_lock.unlock().unwrap();
                dylib_path
            });
            theads.push(t);
        }
        let mut dylib_paths = vec![];
        for t in theads {
            let dylib_path = t.join().unwrap();
            dylib_paths.push(dylib_path);
        }
        dylib_paths
    }
}
