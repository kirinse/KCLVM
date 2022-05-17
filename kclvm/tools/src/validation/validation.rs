use kclvm_ast::{ast::{self, Node, NodeRef}, node_ref};

use serde_json;
use serde_yaml;

use std::{fs::File, collections::HashMap, path::Path};
use kclvm_ast::ast::*;
use kclvm_ast::ast::Identifier;
use kclvm_compiler::{libgen::libgen::DyLibGenerator};
use kclvm_runner::command::Command;


const TEMP_FILE: &str = "validationTempKCLCode.k";
const evaluator: Evaluator = Evaluator{};

pub struct Validator {
    pub validation_data_format: SupportValidationDataFormat,
}

// 两种方案，1.将json和yaml的value保存在一个一样的中间结构中，
// 用一个方法处理这个中间结构就可以了。
// 缺点是后面如果添加一个和json或者yaml的结构差的特别多的，无法使用这种中间结构的就GG了

// 2. 分别为json和yaml创建验证器的实现类，由于yaml和json的结构比较像，
// 缺点是目前会导致重复代码比较多。

// 目前倾向于后者。

pub trait ValidationDataConvert<V, P>{
    fn convert_value_to_ast(&self, schema_name: Option<&str>, value: &V, data_parser: &P) -> NodeRef<Expr>;
}

pub struct JsonConverter;
impl ValidationDataConvert<serde_json::Value, JsonParser> for JsonConverter{
    fn convert_value_to_ast(&self, schema_name: Option<&str>, value: &serde_json::Value, data_parser: &JsonParser) -> NodeRef<Expr> {
        match value{
            serde_json::Value::Null => {
                node_ref!(Expr::NameConstantLit(NameConstantLit {value: NameConstant::None,}))
            }
            serde_json::Value::Bool(j_bool) => {
                node_ref!(Expr::NameConstantLit(NameConstantLit {value: NameConstant::try_from(*j_bool).unwrap()}))
            }
            serde_json::Value::Number(j_num) => {
                if j_num.is_f64() {
                    node_ref!(Expr::NumberLit(NumberLit{binary_suffix: None, value: NumberLitValue::Float(j_num.as_f64().unwrap())}))
                }else if j_num.is_i64(){
                    node_ref!(Expr::NumberLit(NumberLit{binary_suffix: None, value: NumberLitValue::Int(j_num.as_i64().unwrap())}))
                }else{
                    node_ref!(Expr::NumberLit(NumberLit{binary_suffix: None, value: NumberLitValue::Int(j_num.as_u64().unwrap().try_into().unwrap())}))
                }
            }
            serde_json::Value::String(j_string) => {
                node_ref!(Expr::StringLit(StringLit::try_from(j_string.to_string()).unwrap()))
            }
            serde_json::Value::Array(j_arr) => {
                let mut j_arr_ast_nodes:Vec<NodeRef<Expr>> = Vec::new();
                for j_arr_item in j_arr{
                    j_arr_ast_nodes.push(self.convert_value_to_ast(schema_name, &j_arr_item, data_parser));
                }
                node_ref!(Expr::List(ListExpr{
                    ctx: ExprContext::Load,
                    elts: j_arr_ast_nodes
                }))
            }
            serde_json::Value::Object(j_map) => {
                let mut config_entries: Vec<NodeRef<ConfigEntry>> = Vec::new();

                for (k, v) in j_map.iter() {
                    let config_entry = node_ref!(ConfigEntry{
                        key: Some(self.convert_value_to_ast(schema_name, &data_parser.parse_str(k), data_parser)),
                        value: self.convert_value_to_ast(None, v, data_parser),
                        operation: ConfigEntryOperation::Union,
                        insert_index: -1
                    });
                    config_entries.push(config_entry);
                }

                let config_expr = node_ref!(Expr::Config(ConfigExpr{
                    items: config_entries
                }));

                match schema_name{
                    Some(s_name) => {
                        let iden = node_ref!(Identifier{
                            names:vec![s_name.to_string()],
                            pkgpath: String::new(),
                            ctx: ExprContext::Load
                        });
                        node_ref!(Expr::Schema(SchemaExpr{
                            name: iden,
                            config: config_expr,
                            args: vec![],
                            kwargs: vec![]
                        }))
                    },
                    None => {
                        config_expr
                    }
                }
            }
        }
    }
}

pub struct YamlConverter;
impl ValidationDataConvert<serde_yaml::Value, YamlParser> for YamlConverter{
    fn convert_value_to_ast(&self, schema_name: Option<&str>, value: &serde_yaml::Value, data_parser: &YamlParser) -> NodeRef<Expr> {
        match value{
            serde_yaml::Value::Null => {
                node_ref!(Expr::NameConstantLit(NameConstantLit {value: NameConstant::None,}))
            }
            serde_yaml::Value::Bool(j_bool) => {
                node_ref!(Expr::NameConstantLit(NameConstantLit {value: NameConstant::try_from(*j_bool).unwrap()}))
            }
            serde_yaml::Value::Number(j_num) => {
                if j_num.is_f64() {
                    node_ref!(Expr::NumberLit(NumberLit{binary_suffix: None, value: NumberLitValue::Float(j_num.as_f64().unwrap())}))
                }else if j_num.is_i64(){
                    node_ref!(Expr::NumberLit(NumberLit{binary_suffix: None, value: NumberLitValue::Int(j_num.as_i64().unwrap())}))
                }else{
                    node_ref!(Expr::NumberLit(NumberLit{binary_suffix: None, value: NumberLitValue::Int(j_num.as_u64().unwrap().try_into().unwrap())}))
                }
            }
            serde_yaml::Value::String(j_string) => {
                node_ref!(Expr::StringLit(StringLit::try_from(j_string.to_string()).unwrap()))
            }
            serde_yaml::Value::Sequence(j_arr) => {
                let mut j_arr_ast_nodes:Vec<NodeRef<Expr>> = Vec::new();
                for j_arr_item in j_arr{
                    j_arr_ast_nodes.push(self.convert_value_to_ast(schema_name, &j_arr_item, data_parser));
                }
                node_ref!(Expr::List(ListExpr{
                    ctx: ExprContext::Load,
                    elts: j_arr_ast_nodes
                }))
            }
            serde_yaml::Value::Mapping(j_map) => {
                let mut config_entries: Vec<NodeRef<ConfigEntry>> = Vec::new();

                for (k, v) in j_map.iter() {
                    let config_entry = node_ref!(ConfigEntry{
                        key: Some(self.convert_value_to_ast(schema_name, k, data_parser)),
                        value: self.convert_value_to_ast(None, v, data_parser),
                        operation: ConfigEntryOperation::Union,
                        insert_index: -1
                    });
                    config_entries.push(config_entry);
                }

                let config_expr = node_ref!(Expr::Config(ConfigExpr{
                    items: config_entries
                }));

                match schema_name{
                    Some(s_name) => {
                        let iden = node_ref!(Identifier{
                            names:vec![s_name.to_string()],
                            pkgpath: String::new(),
                            ctx: ExprContext::Load
                        });
                        node_ref!(Expr::Schema(SchemaExpr{
                            name: iden,
                            config: config_expr,
                            args: vec![],
                            kwargs: vec![]
                        }))
                    },
                    None => {
                        config_expr
                    }
                }
            }
        }
    }
}

pub trait ValidationDataLoad<V>{
    fn get_value_from_data(&self, data_filepath_or_str: &str) -> V;
}
pub trait Validation<V>{ 
    fn trans_data_to_ast(&self, schema_name: Option<&str>, value: &V) -> NodeRef<Expr>;
    fn get_value(&self, data_filepath_or_str: &str) -> V;
    fn validate(&self, schema_name: Option<&str>, attribute_name: Option<&str>, data_filepath_or_str: &str, kcl_path: Option<&str>, kcl_code: Option<String>) -> bool{
        let value= self.get_value(data_filepath_or_str);
        let ast_node = self.trans_data_to_ast(schema_name, &value);
        let assign = AstUtil::build_assign_node(attribute_name, ast_node);
        let filename = match kcl_path{
            Some(s) => s,
            None => TEMP_FILE,
        };
        let mut ast_m = kclvm_parser::parse_file(filename, kcl_code);
        ast_m.body.push(assign);
        // eval.EvalAST(module)
        evaluator.eval_ast(ast_m);
        return true;
    }
}



//For Json File
pub struct JsonFileValidator{
    json_parser: JsonParser,
    json_converter: JsonConverter
}

impl Validation<serde_json::Value> for JsonFileValidator{
    fn trans_data_to_ast(&self, schema_name: Option<&str>, value: &serde_json::Value) -> NodeRef<Expr> {
        self.json_converter.convert_value_to_ast(schema_name, value, &self.json_parser)
    }

    fn get_value(&self, data_filepath_or_str: &str) -> serde_json::Value {
        self.json_parser.parse_file(data_filepath_or_str)
    }
}

pub struct JsonStrValidator{
    json_parser: JsonParser,
    json_converter: JsonConverter
}

impl Validation<serde_json::Value> for JsonStrValidator{
    fn trans_data_to_ast(&self, schema_name: Option<&str>, value: &serde_json::Value) -> NodeRef<Expr> {
        self.json_converter.convert_value_to_ast(schema_name, value, &self.json_parser)
    }

    fn get_value(&self, data_filepath_or_str: &str) -> serde_json::Value {
        self.json_parser.parse_str(data_filepath_or_str)
    }
}

pub enum SupportValidationDataFormat {
    Yaml(YamlParser),
    Json(JsonParser),
}

// File Parser
pub trait DataFormatParser<T>{
    fn parse_file(&self, file_path: &str) -> T {
        let f = File::open(file_path).unwrap();
        self.load_file(f)
    }
    fn load_file(&self, f: File) -> T;
    fn parse_str(&self, s: &str) -> T;
}

pub struct YamlParser;
impl DataFormatParser<serde_yaml::Value> for YamlParser {
    fn load_file(&self, f: File) -> serde_yaml::Value {
        let v: serde_yaml::Value = serde_yaml::from_reader(f).unwrap();
        v
    }

    fn parse_str(&self, s: &str) -> serde_yaml::Value {
        let v = serde_yaml::to_value(s).unwrap();
        v
    }
}

pub struct JsonParser;
impl DataFormatParser<serde_json::Value> for JsonParser {
    fn load_file(&self, f: File) -> serde_json::Value {
        let v: serde_json::Value = serde_json::from_reader(f).unwrap();
        v
    }

    fn parse_str(&self, s: &str) -> serde_json::Value {
        let v = serde_json::to_value(s).unwrap();
        v
    }
}

// kcl eval
trait Eval<T>{
    fn eval_code(&self, file_name: &str, code: Option<String>);
    fn eval_ast(&self, module: T);
    fn build_and_run_ast(&self, module: T);
}

const MAIN_PKG_NAME: &str = "__main__";
pub struct Evaluator;

impl Eval<ast::Module> for Evaluator{
    fn eval_code(&self, file_name: &str, code: Option<String>){
        let module = kclvm_parser::parse_file(file_name, code);
        self.eval_ast(module);
    }

    fn eval_ast(&self, mut module: ast::Module){
        module.pkg = MAIN_PKG_NAME.to_string();
        self.build_and_run_ast(module);
    }

    fn build_and_run_ast(&self, module: ast::Module){
        let mut pkgs_ast = HashMap::new();
        pkgs_ast.insert(MAIN_PKG_NAME.to_string(), vec![module]);
        // load ast
        let mut program = ast::Program{
            root: MAIN_PKG_NAME.to_string(),
            main: MAIN_PKG_NAME.to_string(),
            pkgs: pkgs_ast,
            cmd_args: vec![],
            cmd_overrides: vec![]
        };
        let dylib_paths = DyLibGenerator::gen_and_run_dylib_from_ast(program);
        // let dylib_paths = dylib_gen.gen
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


//AST Utils
