use kclvm_fileutils::file_utils::DataFileParser;
use kclvm_fileutils::file_utils::ValidationDataConvert;
use kclvm_fileutils::json_utils::JsonConverter;
use kclvm_fileutils::json_utils::JsonParser;
use kclvm_fileutils::yaml_utils::YamlConverter;
use kclvm_fileutils::yaml_utils::YamlParser;
use kclvm_runner::eval::Evaluator;
use serde_json;
use kclvm_ast::ast::*;
use kclvm_ast::ast_utils::*;

const TEMP_FILE: &str = "validationTempKCLCode.k";

pub trait Validation<V>{ 
    fn eval_module(&self, ast_m: Module);
    fn trans_data_to_ast(&self, schema_name: Option<&str>, value: &V) -> NodeRef<Expr>;
    fn get_value(&self, data_filepath_or_str: &str) -> V;
    fn validate(&self, schema_name: Option<&str>, attribute_name: Option<&str>, data_filepath_or_str: &str, kcl_path: Option<&str>, kcl_code: Option<String>) -> bool{
        let value= self.get_value(data_filepath_or_str);
        let ast_node = self.trans_data_to_ast(schema_name, &value);
        let attribute_name = match attribute_name {
            Some(attr_name) => attr_name,
            None => "value",
        };
        println!("attrname: {}",attribute_name);
        let assign = AstUtil::build_assign_node(attribute_name, ast_node);
        let filename = match kcl_path{
            Some(s) => s,
            None => TEMP_FILE,
        };
        let mut ast_m = kclvm_parser::parse_file(filename, kcl_code);
        ast_m.body.push(assign);
        // eval.EvalAST(module)
        self.eval_module(ast_m);
        return true;
    }
}

//For Json
pub struct JsonFileValidator{
    json_parser: JsonParser,
    json_converter: JsonConverter,
    ast_module_eval: Evaluator
}

impl JsonFileValidator{
    pub fn new() -> Self{
        Self { 
            json_parser: JsonParser{}, 
            json_converter: JsonConverter{}, 
            ast_module_eval: Evaluator{} 
        }
    }
}

impl Validation<serde_json::Value> for JsonFileValidator{
    fn trans_data_to_ast(&self, schema_name: Option<&str>, value: &serde_json::Value) -> NodeRef<Expr> {
        self.json_converter.convert_value_to_ast(schema_name, value, &self.json_parser)
    }

    fn get_value(&self, data_filepath_or_str: &str) -> serde_json::Value {
        self.json_parser.parse_file(data_filepath_or_str)
    }

    fn eval_module(&self, ast_m: Module) {
        self.ast_module_eval.eval_main_module(ast_m);
    }
}

pub struct JsonStrValidator{
    json_parser: JsonParser,
    json_converter: JsonConverter,
    ast_module_eval: Evaluator
}

impl JsonStrValidator{
    pub fn new() -> Self{
        Self { 
            json_parser: JsonParser{}, 
            json_converter: JsonConverter{}, 
            ast_module_eval: Evaluator{} 
        }
    }
}

impl Validation<serde_json::Value> for JsonStrValidator{
    fn trans_data_to_ast(&self, schema_name: Option<&str>, value: &serde_json::Value) -> NodeRef<Expr> {
        self.json_converter.convert_value_to_ast(schema_name, value, &self.json_parser)
    }

    fn get_value(&self, data_filepath_or_str: &str) -> serde_json::Value {
        self.json_parser.parse_str(data_filepath_or_str)
    }

    fn eval_module(&self, ast_m: Module) {
        self.ast_module_eval.eval_main_module(ast_m);
    }
}

//For Yaml
pub struct YamlFileValidator{
    yaml_parser: YamlParser,
    yaml_converter: YamlConverter,
    ast_module_eval: Evaluator
}

impl YamlFileValidator{
    pub fn new() -> Self{
        Self { 
            yaml_parser: YamlParser{}, 
            yaml_converter: YamlConverter{}, 
            ast_module_eval: Evaluator{} 
        }
    }
}

impl Validation<serde_yaml::Value> for YamlFileValidator{
    fn trans_data_to_ast(&self, schema_name: Option<&str>, value: &serde_yaml::Value) -> NodeRef<Expr> {
        self.yaml_converter.convert_value_to_ast(schema_name, value, &self.yaml_parser)
    }

    fn get_value(&self, data_filepath_or_str: &str) -> serde_yaml::Value {
        self.yaml_parser.parse_file(data_filepath_or_str)
    }

    fn eval_module(&self, ast_m: Module) {
        self.ast_module_eval.eval_main_module(ast_m);
    }
}

pub struct YamlStrValidator{
    yaml_parser: YamlParser,
    yaml_converter: YamlConverter,
    ast_module_eval: Evaluator
}

impl YamlStrValidator{
    pub fn new() -> Self{
        Self { 
            yaml_parser: YamlParser{}, 
            yaml_converter: YamlConverter{}, 
            ast_module_eval: Evaluator{} 
        }
    }
}

impl Validation<serde_yaml::Value> for YamlStrValidator{
    fn trans_data_to_ast(&self, schema_name: Option<&str>, value: &serde_yaml::Value) -> NodeRef<Expr> {
        self.yaml_converter.convert_value_to_ast(schema_name, value, &self.yaml_parser)
    }

    fn get_value(&self, data_filepath_or_str: &str) -> serde_yaml::Value {
        self.yaml_parser.parse_str(data_filepath_or_str)
    }

    fn eval_module(&self, ast_m: Module) {
        self.ast_module_eval.eval_main_module(ast_m);
    }
}