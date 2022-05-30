use super::{r#override::apply_override_on_module, *};
use crate::printer::print_ast_module;
use kclvm_ast::ast;
use kclvm_parser::parse_file;
use pretty_assertions::assert_eq;

#[test]
fn test_override_file_simple() {
    let specs = vec![
        "config.image=\"image/image\"".to_string(),
        ":config.image=\"image/image:v1\"".to_string(),
        ":config.data={id=1,value=\"override_value\"}".to_string(),
    ];
    let import_paths = vec![];
    assert_eq!(
        override_file("./src/query/test_data/simple.k", &specs, &import_paths).unwrap(),
        true
    )
}

#[test]
fn test_override_file_import_paths() {
    let specs = vec!["data.value=\"override_value\"".to_string()];
    let import_paths = vec!["pkg".to_string(), "pkg.pkg".to_string()];
    assert_eq!(
        override_file(
            "./src/query/test_data/import_paths.k",
            &specs,
            &import_paths
        )
        .unwrap(),
        true
    )
}

#[test]
fn test_override_file_config() {
    let specs = vec![
        "appConfiguration.image=kusion/kusion:v0.3.1".to_string(),
        "appConfiguration.mainContainer.name=kusion_override".to_string(),
        "appConfiguration.labels.key.key=override_value".to_string(),
        "appConfiguration.overQuota=False".to_string(),
        "appConfiguration.probe={periodSeconds=20}".to_string(),
        "appConfigurationUnification.image=kusion/kusion:v0.3.1".to_string(),
        "appConfigurationUnification.mainContainer.name=kusion_override".to_string(),
        "appConfigurationUnification.labels.key.key=override_value".to_string(),
        "appConfigurationUnification.overQuota=False".to_string(),
        "appConfigurationUnification.resource-".to_string(),
    ];
    let overrides = specs
        .iter()
        .map(|s| spec_str_to_override(s))
        .filter_map(Result::ok)
        .collect::<Vec<ast::OverrideSpec>>();
    let import_paths = vec![];
    let mut module = parse_file("./src/query/test_data/config.k", None).unwrap();
    for o in &overrides {
        apply_override_on_module(&mut module, o, &import_paths).unwrap();
    }
    let expected_code = print_ast_module(&module);
    assert_eq!(
        expected_code,
        r#"schema Main:
    name?: str
    env?: [{str:}]

schema Probe:
    initialDelaySeconds?: int
    timeoutSeconds?: int
    periodSeconds?: int = 10
    successThreshold?: int
    failureThreshold?: int

schema AppConfiguration:
    appName: str
    image: str
    overQuota: bool = False
    resource: {str:}
    mainContainer?: Main
    labels: {str:}
    probe?: Probe

appConfiguration = AppConfiguration {
    appName: "kusion"
    image: "kusion/kusion:v0.3.1"
    resource: {
        cpu: "4"
        disk: "50Gi"
        memory: "12Gi"
    }
    labels: {key: {key: "override_value"}}
    mainContainer: Main {name: "kusion_override"}
    overQuota = False
    overQuota = False
    probe: {periodSeconds = 20}
}

appConfigurationUnification: AppConfiguration {
    appName: "kusion"
    image: "kusion/kusion:v0.3.1"
    labels: {key: {key: "override_value"}}
    mainContainer: Main {name: "kusion_override"}
    overQuota: False
}
"#
    );
}

#[test]
fn test_spec_str_to_override_invalid() {
    let specs = vec![":a:", "=a=", ":a", "a-1"];
    for spec in specs {
        assert!(spec_str_to_override(spec).is_err(), "{} test failed", spec);
    }
}
