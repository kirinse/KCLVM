
use kclvm_fileutils::json_utils::{JsonParser, JsonConverter};
use kclvm_runner::eval::Evaluator;
use serde_json;
use serde_json::{Result, Value};
use kclvm_ast::ast::*;
use kclvm_parser::parse_file;
use std::fs::File;

use super::validation::{JsonFileValidator, JsonStrValidator, Validation};

const JSON_TEST_DATA_PATH: &str = "json_test_data";
const JSON_INVALID_TEST_DATA_PATH: &str = "json_invalid_test_data";
const KCL_FILE_SUFFIX: &str = ".k";
const JSON_FILE_SUFFIX: &str = ".k.json";
const INVALID_TEST_CASES: &[&'static str; 3] = &[
    "schema_with_check",
    "schema",
    "simple",
];

const TEST_CASES: &[&'static str; 5] = &[
    "complex",
    "list",
    "plain_value",
    "schema_with_check",
    "simple",
];

fn read_json(file_name: &str, test_case_type: &str) -> String{
    let f = File::open(&format!("./src/validation/test_data/{}/{}{}", test_case_type, file_name, JSON_FILE_SUFFIX),).unwrap();
    let v: serde_json::Value = serde_json::from_reader(f).unwrap();
    v.to_string()
}

fn read_kcl(file_name: &str, test_case_type: &str) -> Module{
    let module = parse_file(
        &format!("./src/validation/test_data/{}/{}{}", test_case_type, file_name, KCL_FILE_SUFFIX),
        None,
    );
    module
}
#[test]
fn test_validate_code_normal_json_data(){
    let json_str_validator = JsonStrValidator::new();

    for case in TEST_CASES {
        let json_str = read_json(&case, JSON_TEST_DATA_PATH);
        let kcl_path = &format!("./src/validation/test_data/{}/{}{}", JSON_TEST_DATA_PATH, case, KCL_FILE_SUFFIX);
        let result = json_str_validator.validate(None, None, &json_str, Some(kcl_path), None);
        assert!(result);
        println!("{} is OK !",case)
    }
}

