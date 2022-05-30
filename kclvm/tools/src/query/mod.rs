//! This package is mainly the implementation of the KCL query tool, mainly including 
//! KCL code modification `override` and other implementations. We can call the `override_file` 
//! function to modify the file. The main principle is to parse the AST according to the 
//! input file name, and according to the ast: :OverrideSpec transforms the nodes in the 
//! AST, recursively modifying or deleting the values of the nodes in the AST.
pub mod r#override;

#[cfg(test)]
mod tests;

pub use r#override::{apply_overrides, override_file, spec_str_to_override};
