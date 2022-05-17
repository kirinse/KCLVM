use crate::validation::{self, validation::FileLoader};
use serde_json;
use serde_json::{Result, Value};

#[test]
fn test_json_loader(){
    let json_file_path = "/Users/shijun/Workspace/KCLVM/kclvm/tools/src/validation/test.json";
    println!("---------------------Json Loader Begin-------------------");
    let jl = validation::validation::JsonLoader{};
    let v:serde_json::Value = jl.open_file(json_file_path.to_string());
    println!("{:?}", v);
    println!("---------------------Json Loader End-------------------");


    println!("---------------------Yaml Loader Begin-------------------");
    let yaml_file_path="/Users/shijun/Workspace/KCLVM/kclvm/tools/src/validation/test.yaml";
    let yl = validation::validation::YamlLoader{};
    let v:serde_yaml::Value = yl.open_file(yaml_file_path.to_string());
    println!("{:?}", v);
    println!("---------------------Yaml Loader End-------------------");

    let json_str = "{
        \"name\": \"John Doe\",
        \"age\": 43,
        \"address\": {
            \"street\": \"10 Downing Street\",
            \"city\": \"London\"
        },
        \"phones\": [
            \"+44 1234567\",
            \"+44 2345678\"
        ]
    }";

    let v = serde_json::to_value(json_str).unwrap();

}