// Copyright 2021 The KCL Authors. All rights reserved.

use std::rc::Rc;

use crate::*;

pub const SETTINGS_OUTPUT_KEY: &str = "output_type";
pub const SETTINGS_SCHEMA_TYPE_KEY: &str = "__schema_type__";
pub const SETTINGS_OUTPUT_STANDALONE: &str = "STANDALONE";
pub const SETTINGS_OUTPUT_INLINE: &str = "INLINE";
pub const SETTINGS_OUTPUT_IGNORE: &str = "IGNORE";
pub const SCHEMA_SETTINGS_ATTR_NAME: &str = "__settings__";
pub const CONFIG_META_FILENAME: &str = "$filename";
pub const CONFIG_META_LINE: &str = "$lineno";
pub const CONFIG_META_COLUMN: &str = "$columnno";
pub const CONFIG_ITEM_META_FILENAME: &str = "filename";
pub const CONFIG_ITEM_META_LINE: &str = "lineno";
pub const CONFIG_ITEM_META_COLUMN: &str = "columnno";
pub const CONFIG_ITEM_META: &str = "$config_meta";
pub const MAIN_PKG_PATH: &str = "__main__";
pub const PKG_PATH_PREFIX: char = '@';
pub const CAL_MAP_RUNTIME_TYPE: &str = "cal_map_runtime_type";
pub const CAL_MAP_META_LINE: &str = "cal_map_meta_line";

/// Get the schema runtime type use the schema name and pkgpath
pub fn schema_runtime_type(name: &str, pkgpath: &str) -> String {
    format!("{}.{}", pkgpath, name)
}

/// Construct a schema config meta dict using filename, line and column
#[inline]
pub fn schema_config_meta(filename: &str, line: u64, column: u64) -> ValueRef {
    ValueRef::dict(Some(&[
        (CONFIG_META_FILENAME, &ValueRef::str(filename)),
        (CONFIG_META_LINE, &ValueRef::int(line as i64)),
        (CONFIG_META_COLUMN, &ValueRef::int(column as i64)),
    ]))
}

impl ValueRef {
    pub fn dict_to_schema(&self, name: &str, pkgpath: &str, config_keys: &[String]) -> Self {
        if self.is_dict() {
            Self::from(Value::schema_value(Box::new(SchemaValue {
                name: name.to_string(),
                pkgpath: pkgpath.to_string(),
                config: Rc::new(self.as_dict_ref().clone()),
                config_keys: config_keys.to_owned(),
            })))
        } else if self.is_schema() {
            self.clone()
        } else {
            panic!("invalid dict object to schema")
        }
    }

    pub fn schema_to_dict(&self) -> Self {
        match &*self.rc {
            Value::schema_value(ref schema) => {
                Self::from(Value::dict_value(Box::new(schema.config.as_ref().clone())))
            }
            Value::dict_value(_) => self.clone(),
            _ => panic!("invalid schema object to dict"),
        }
    }

    pub fn schema_check_attr_optional(
        &self,
        optional_mapping: &ValueRef,
        schema_name: &str,
        config_meta: &ValueRef,
    ) {
        let attr_map = match &*self.rc {
            Value::schema_value(ref schema) => {
                let schema: &mut SchemaValue = get_ref_mut(schema);
                let schema = get_ref_mut(schema);
                &schema.config.values
            }
            Value::dict_value(ref schema) => {
                let schema: &mut DictValue = get_ref_mut(schema);
                &schema.values
            }
            _ => panic!("Invalid schema/dict value, got {}", self.type_str()),
        };
        match &*optional_mapping.rc {
            Value::dict_value(ref optional_mapping) => {
                let optional_mapping = get_ref_mut(optional_mapping);
                for (attr, is_optional) in &optional_mapping.values {
                    let is_required = !is_optional.as_bool();
                    let undefined = ValueRef::undefined();
                    let value = attr_map.get(attr).unwrap_or(&undefined);
                    if is_required && value.is_none_or_undefined() {
                        let filename = config_meta.get_by_key(CONFIG_META_FILENAME);
                        let line = config_meta.get_by_key(CONFIG_META_LINE);
                        let ctx = Context::current_context_mut();
                        if let Some(filename) = filename {
                            ctx.set_kcl_filename(&filename.as_str());
                        }
                        if let Some(line) = line {
                            ctx.panic_info.kcl_line = line.as_int() as i32;
                        }
                        panic!(
                            "attribute '{}' of {} is required and can't be None or Undefined",
                            attr, schema_name
                        );
                    }
                }
            }
            _ => panic!(
                "Invalid optional mapping, got {}",
                optional_mapping.type_str()
            ),
        }
    }

    pub fn schema_default_settings(&mut self, config: &ValueRef, runtime_type: &str) {
        let settings = self.dict_get_value(SCHEMA_SETTINGS_ATTR_NAME);
        if settings.is_none() || (settings.is_some() && !settings.unwrap().is_config()) {
            let mut default_settings = ValueRef::dict(None);
            default_settings
                .dict_update_key_value(SETTINGS_OUTPUT_KEY, ValueRef::str(SETTINGS_OUTPUT_INLINE));
            default_settings
                .dict_update_key_value(SETTINGS_SCHEMA_TYPE_KEY, ValueRef::str(runtime_type));
            self.dict_update_key_value(SCHEMA_SETTINGS_ATTR_NAME, default_settings);
        } else {
            let settings = get_ref_mut(settings.unwrap());
            settings.dict_update_key_value(SETTINGS_SCHEMA_TYPE_KEY, ValueRef::str(runtime_type));
        }
        if let Some(v) = config.dict_get_value(SCHEMA_SETTINGS_ATTR_NAME) {
            self.dict_update_key_value(SCHEMA_SETTINGS_ATTR_NAME, v.clone());
        }
    }

    pub fn attr_str(&self) -> String {
        match &*self.rc {
            Value::int_value(v) => v.to_string(),
            Value::float_value(v) => v.to_string(),
            Value::str_value(v) => v.clone(),
            _ => panic!("invalid attribute {}", self.type_str()),
        }
    }

    pub fn update_attr_map(&mut self, name: &str, type_str: &str) {
        match &*self.rc {
            Value::dict_value(dict) => {
                let attr_map = get_ref_mut(&dict.attr_map);
                attr_map.insert(name.to_string(), type_str.to_string());
            }
            Value::schema_value(schema) => {
                let attr_map = get_ref_mut(&schema.config.attr_map);
                attr_map.insert(name.to_string(), type_str.to_string());
            }
            _ => panic!("invalid object '{}' in update_attr_map", self.type_str()),
        }
    }

    pub fn attr_map_get(&mut self, name: &str) -> Option<&String> {
        match &*self.rc {
            Value::dict_value(dict) => {
                let attr_map = get_ref_mut(&dict.attr_map);
                attr_map.get(name)
            }
            Value::schema_value(schema) => {
                let attr_map = get_ref_mut(&schema.config.attr_map);
                attr_map.get(name)
            }
            _ => panic!("invalid object '{}' in attr_map_get", self.type_str()),
        }
    }

    pub fn schema_update_with_schema(&mut self, value: &ValueRef) {
        if let (Value::schema_value(schema), Value::schema_value(value)) = (&*self.rc, &*value.rc) {
            let values = get_ref_mut(&schema.config.values);
            let ops = get_ref_mut(&schema.config.ops);
            let insert_indexs = get_ref_mut(&schema.config.insert_indexs);
            for (k, v) in &value.config.values {
                let op = value
                    .config
                    .ops
                    .get(k)
                    .unwrap_or(&ConfigEntryOperationKind::Union);
                let index = value.config.insert_indexs.get(k).unwrap_or(&-1);
                values.insert(k.clone(), v.clone());
                ops.insert(k.clone(), op.clone());
                insert_indexs.insert(k.clone(), *index);
            }
        }
    }
}

#[cfg(test)]
mod test_value_schema {
    use crate::*;

    const TEST_SCHEMA_NAME: &str = "Data";

    fn get_test_schema_value() -> ValueRef {
        let config = ValueRef::dict(None);
        let mut schema = ValueRef::dict(None).dict_to_schema(TEST_SCHEMA_NAME, MAIN_PKG_PATH, &[]);
        schema.schema_default_settings(
            &config,
            &schema_runtime_type(TEST_SCHEMA_NAME, MAIN_PKG_PATH),
        );
        schema
    }

    #[test]
    fn test_dict_schema_convention() {
        let dict = ValueRef::dict(None);
        let dict = dict.schema_to_dict();
        assert!(dict.is_dict());
        let schema = dict.dict_to_schema(TEST_SCHEMA_NAME, MAIN_PKG_PATH, &[]);
        assert!(schema.is_schema());
        let schema = schema.dict_to_schema(TEST_SCHEMA_NAME, MAIN_PKG_PATH, &[]);
        assert!(schema.is_schema());
        let dict = schema.schema_to_dict();
        assert!(dict.is_dict());
    }

    #[test]
    fn test_schema_check_attr_optional() {
        let dict = ValueRef::dict_str(&[("key", "value")]);
        let schema = dict.dict_to_schema(TEST_SCHEMA_NAME, MAIN_PKG_PATH, &[]);
        let config_meta = ValueRef::dict(None);
        let optional_mapping = ValueRef::dict_bool(&[("key", true)]);
        schema.schema_check_attr_optional(&optional_mapping, TEST_SCHEMA_NAME, &config_meta);
        let optional_mapping = ValueRef::dict_bool(&[("key", false)]);
        schema.schema_check_attr_optional(&optional_mapping, TEST_SCHEMA_NAME, &config_meta);
        let optional_mapping = ValueRef::dict_bool(&[("another_key", true)]);
        schema.schema_check_attr_optional(&optional_mapping, TEST_SCHEMA_NAME, &config_meta);
    }

    #[test]
    fn test_schema_check_attr_optional_invalid() {
        let err = std::panic::catch_unwind(|| {
            let dict = ValueRef::dict_str(&[("key", "value")]);
            let schema = dict.dict_to_schema(TEST_SCHEMA_NAME, MAIN_PKG_PATH, &[]);
            let config_meta = ValueRef::dict(None);
            let optional_mapping = ValueRef::dict_bool(&[("another_key", false)]);
            schema.schema_check_attr_optional(&optional_mapping, TEST_SCHEMA_NAME, &config_meta);
        });
        assert!(err.is_err())
    }

    #[test]
    fn test_schema_default_settings() {
        let schema = get_test_schema_value();
        let schema_settings = schema.get_by_key(SCHEMA_SETTINGS_ATTR_NAME).unwrap();
        let output_type = schema_settings
            .get_by_key(SETTINGS_OUTPUT_KEY)
            .unwrap()
            .as_str();
        assert_eq!(output_type, SETTINGS_OUTPUT_INLINE);
    }

    #[test]
    fn test_schema_attr_map() {
        let mut schema = get_test_schema_value();
        let entries = [("key1", "str"), ("key2", "int"), ("key3", "str|int")];
        for (attr, type_str) in entries {
            schema.update_attr_map(attr, type_str);
        }
        for (attr, type_str) in entries {
            let result = schema.attr_map_get(attr).unwrap().clone();
            assert_eq!(result, type_str);
        }
    }

    #[test]
    fn test_schema_update_with_schema() {
        let mut schema1 = get_test_schema_value();
        let mut schema2 = get_test_schema_value();
        let entries = [("key1", "value1"), ("key2", "value2")];
        for (key, val) in entries {
            schema2.dict_update_entry(
                key,
                &ValueRef::str(val),
                &ConfigEntryOperationKind::Union,
                &-1,
            );
        }
        assert_ne!(schema1, schema2);
        schema1.schema_update_with_schema(&schema2);
        assert_eq!(schema1, schema2);
    }
}
