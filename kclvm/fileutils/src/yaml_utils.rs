use std::fs::File;

use kclvm_ast::{ast::*, node_ref};

use crate::file_utils::{DataFileParser, ValidationDataConvert};

pub struct YamlParser;
impl DataFileParser<serde_yaml::Value> for YamlParser {
    fn load_file(&self, f: File) -> serde_yaml::Value {
        let v: serde_yaml::Value = serde_yaml::from_reader(f).unwrap();
        v
    }

    fn parse_str(&self, s: &str) -> serde_yaml::Value {
        let v = serde_yaml::to_value(s).unwrap();
        v
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