use std::fs::File;

use kclvm_ast::ast::*;

pub trait DataFileParser<T>{
    fn parse_file(&self, file_path: &str) -> T {
        let f = File::open(file_path).unwrap();
        self.load_file(f)
    }
    fn load_file(&self, f: File) -> T;
    fn parse_str(&self, s: &str) -> T;
}

pub trait ValidationDataConvert<V, P>{
    fn convert_value_to_ast(&self, schema_name: Option<&str>, value: &V, data_parser: &P) -> NodeRef<Expr>;
}