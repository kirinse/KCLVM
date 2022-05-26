use anyhow::{anyhow, Result};
use std::path::Path;
use walkdir::WalkDir;

use crate::printer::print_ast_module;
use kclvm_parser::parse_file;

#[cfg(test)]
mod tests;

const KCL_FILE_PATTERN: &str = ".k";

/// Format options
#[derive(Debug, Default)]
pub struct FormatOptions {
    pub is_stdout: bool,
    pub recursively: bool,
}

/// Formats kcl file or directory path contains kcl files and
/// returns the changed file paths.
pub fn format<P: AsRef<Path>>(path: P, opts: &FormatOptions) -> Result<Vec<String>> {
    let mut changed_paths: Vec<String> = vec![];
    let path_ref = path.as_ref();
    if path_ref.is_dir() {
        for file in &get_kcl_files(path, opts.recursively)? {
            if format_file(file, opts)? {
                changed_paths.push(file.clone())
            }
        }
    } else if path_ref.is_file() {
        let file = path_ref.to_str().unwrap().to_string();
        if format_file(&file, opts)? {
            changed_paths.push(file)
        }
    }
    if !opts.is_stdout {
        let n = changed_paths.len();
        println!(
            "KCL format done and {} {} formatted:",
            n,
            if n <= 1 { "file was" } else { "files were" }
        );
        for p in &changed_paths {
            println!("{}", p);
        }
    }
    Ok(changed_paths)
}

/// Format a code source and return the formatted source and
/// whether the source is changed.
pub fn format_source(src: &str) -> Result<(String, bool)> {
    let module = match parse_file("", Some(src.to_string())) {
        Ok(module) => module,
        Err(err) => return Err(anyhow!("{}", err)),
    };
    let formatted_src = print_ast_module(&module);
    let is_formatted = src != formatted_src;
    Ok((formatted_src, is_formatted))
}

/// Format a file and return
fn format_file(file: &str, opts: &FormatOptions) -> Result<bool> {
    let src = std::fs::read_to_string(file)?;
    let (source, is_formatted) = format_source(&src)?;
    if opts.is_stdout {
        println!("{}", source);
    } else {
        std::fs::write(file, &source)?
    }
    Ok(is_formatted)
}

/// Get kcl files from path.
fn get_kcl_files<P: AsRef<Path>>(path: P, recursively: bool) -> Result<Vec<String>> {
    let mut files = vec![];
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let file = path.to_str().unwrap();
            if file.ends_with(KCL_FILE_PATTERN) && (recursively || entry.depth() == 1) {
                files.push(file.to_string())
            }
        }
    }
    Ok(files)
}
