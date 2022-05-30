use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};

use kclvm_ast::walker::MutSelfMutWalker;
use kclvm_ast::{ast, walk_if_mut};
use kclvm_parser::{parse_expr, parse_file};

use crate::printer::print_ast_module;

/// Override and rewrite a file with override spec.
///
/// # Parameters
///
/// `file`: [&str]
///     The File that need to be overridden
///
/// `specs`: &\[[String]\]
///     List of specs that need to be overridden.
///     Each spec string satisfies the form: <pkgpath>:<field_path>=<filed_value> or <pkgpath>:<field_path>-
///     When the pkgpath is '__main__', it can be omitted.
///
/// `import_paths`: &\[[String]\]
///     List of import paths that are need to be added.
///
/// # Returns
///
/// result: [Result<bool>]
///     Whether override is successful
pub fn override_file(file: &str, specs: &[String], import_paths: &[String]) -> Result<bool> {
    let overrides = specs
        .iter()
        .map(|s| spec_str_to_override(s))
        .filter_map(Result::ok)
        .collect::<Vec<ast::OverrideSpec>>();
    let mut module = match parse_file(file, None) {
        Ok(module) => module,
        Err(msg) => return Err(anyhow!("{}", msg)),
    };
    let mut result = false;
    for o in &overrides {
        if apply_override_on_module(&mut module, o, import_paths)? {
            result = true;
        }
    }
    if result {
        let code_str = print_ast_module(&module);
        std::fs::write(file, &code_str)?
    }
    Ok(result)
}

/// Override spec string to override structure
pub fn spec_str_to_override(spec: &str) -> Result<ast::OverrideSpec> {
    let err = Err(anyhow!("Invalid spec format '{}', expected <pkgpath>:<field_path>=<filed_value> or <pkgpath>:<field_path>-", spec));
    if spec.contains('=') {
        // Create or update the override value.
        let split_values = spec.splitn(2, '=').collect::<Vec<&str>>();
        let paths = split_values[0].splitn(2, ':').collect::<Vec<&str>>();
        if split_values.len() < 2 || paths.len() > 2 {
            err
        } else {
            let (pkgpath, field_path) = if paths.len() == 1 {
                ("".to_string(), paths[0].to_string())
            } else {
                (paths[0].to_string(), paths[1].to_string())
            };
            if field_path.is_empty() || split_values[1].is_empty() {
                err
            } else {
                Ok(ast::OverrideSpec {
                    pkgpath,
                    field_path,
                    field_value: split_values[1].to_string(),
                    action: ast::OverrideAction::CreateOrUpdate,
                })
            }
        }
    } else if let Some(stripped_spec) = spec.strip_suffix('-') {
        // Delete the override value.
        let paths = stripped_spec.splitn(2, ':').collect::<Vec<&str>>();
        if paths.len() > 2 {
            err
        } else {
            let (pkgpath, field_path) = if paths.len() == 1 {
                ("".to_string(), paths[0].to_string())
            } else {
                (paths[0].to_string(), paths[1].to_string())
            };
            if field_path.is_empty() {
                err
            } else {
                Ok(ast::OverrideSpec {
                    pkgpath,
                    field_path,
                    field_value: "".to_string(),
                    action: ast::OverrideAction::Delete,
                })
            }
        }
    } else {
        err
    }
}

/// Apply overrides on the AST program with the override specifications.
pub fn apply_overrides(
    prog: &mut ast::Program,
    overrides: &[ast::OverrideSpec],
    import_paths: &[String],
) -> Result<()> {
    for o in overrides {
        let pkgpath = if o.pkgpath.is_empty() {
            &prog.main
        } else {
            &o.pkgpath
        };
        if let Some(modules) = prog.pkgs.get_mut(pkgpath) {
            for m in modules.iter_mut() {
                if apply_override_on_module(m, o, import_paths)? {
                    let code_str = print_ast_module(m);
                    std::fs::write(&m.filename, &code_str)?
                }
            }
        }
    }
    Ok(())
}

/// Apply overrides on the AST module with the override specifications.
pub fn apply_override_on_module(
    m: &mut ast::Module,
    o: &ast::OverrideSpec,
    import_paths: &[String],
) -> Result<bool> {
    if !import_paths.is_empty() {
        let mut exist_import_set: HashSet<String> = HashSet::new();
        for stmt in &m.body {
            if let ast::Stmt::Import(import_stmt) = &stmt.node {
                exist_import_set.insert(import_stmt.path.to_string());
            }
        }
        for (i, path) in import_paths.iter().enumerate() {
            let line: u64 = i as u64 + 1;
            if exist_import_set.contains(path) {
                continue;
            }
            let name = path
                .split('.')
                .last()
                .ok_or_else(|| anyhow!("invalid import path {}", path))?;
            let import_node = ast::ImportStmt {
                path: path.to_string(),
                rawpath: "".to_string(),
                name: name.to_string(),
                asname: None,
            };
            let import_stmt = Box::new(ast::Node::new(
                ast::Stmt::Import(import_node),
                m.filename.clone(),
                line,
                1,
                line,
                7 + path.len() as u64,
            ));
            m.body.insert((line - 1) as usize, import_stmt)
        }
    }
    let ss = o.field_path.split('.').collect::<Vec<&str>>();
    if ss.len() <= 1 {
        Ok(false)
    } else {
        let target_id = ss[0];
        let field = ss[1..].join(".");
        let value = &o.field_value;
        let key = ast::Identifier {
            names: field.split('.').map(|s| s.to_string()).collect(),
            ctx: ast::ExprContext::Store,
            pkgpath: "".to_string(),
        };
        fix_multi_assign(m);
        let val = build_expr_from_string(value);
        let mut transformer = OverrideTransformer {
            target_id: target_id.to_string(),
            field_path: field,
            override_key: key,
            override_value: val,
            override_target_count: 0,
            has_override: false,
            action: o.action.clone(),
        };
        transformer.walk_module(m);
        Ok(transformer.has_override)
    }
}

/// Build a expression from string
fn build_expr_from_string(value: &str) -> ast::NodeRef<ast::Expr> {
    if value.is_empty() {
        Box::new(ast::Node::dummy_node(ast::Expr::StringLit(
            ast::StringLit {
                is_long_string: false,
                raw_value: "\"\"".to_string(),
                value: "".to_string(),
            },
        )))
    } else {
        let expr = parse_expr(value);
        match &expr.node {
            ast::Expr::Identifier(_) | ast::Expr::Binary(_) | ast::Expr::If(_) => Box::new(
                ast::Node::dummy_node(ast::Expr::StringLit(ast::StringLit {
                    is_long_string: false,
                    raw_value: format!("{:?}", value),
                    value: value.to_string(),
                })),
            ),
            _ => expr,
        }
    }
}

/// Transform AST and fix multi assign statement.
fn fix_multi_assign(m: &mut ast::Module) {
    let mut transformer = MultiAssignTransformer::default();
    transformer.walk_module(m);
    for (offset, (index, assign_stmt)) in transformer.multi_assign_mapping.iter().enumerate() {
        let insert_index = index + offset;
        let pos = m.body[insert_index].pos().clone();
        m.body.insert(
            insert_index,
            Box::new(ast::Node::node_with_pos(
                ast::Stmt::Assign(assign_stmt.clone()),
                pos,
            )),
        );
    }
}

#[derive(Debug, Default)]
struct MultiAssignTransformer {
    pub multi_assign_mapping: HashMap<usize, ast::AssignStmt>,
    pub index: usize,
}

impl<'ctx> MutSelfMutWalker<'ctx> for MultiAssignTransformer {
    fn walk_assign_stmt(&mut self, assign_stmt: &'ctx mut ast::AssignStmt) {
        self.index += 1;
        if assign_stmt.targets.len() <= 1 {
            return;
        }
        for target in &assign_stmt.targets[1..] {
            let mut new_assign_stmt = assign_stmt.clone();
            new_assign_stmt.targets = vec![target.clone()];
            self.multi_assign_mapping
                .insert(self.index, new_assign_stmt);
        }
        assign_stmt.targets = vec![assign_stmt.targets[0].clone()];
    }
    fn walk_if_stmt(&mut self, _: &'ctx mut ast::IfStmt) {
        // Do not fix AssignStmt in IfStmt
    }
    fn walk_schema_stmt(&mut self, _: &'ctx mut ast::SchemaStmt) {
        // Do not fix AssignStmt in SchemaStmt
    }
    fn walk_lambda_expr(&mut self, _: &'ctx mut ast::LambdaExpr) {
        // Do not fix AssignStmt in LambdaExpr
    }
}

/// OverrideTransformer is used to walk AST and transform it with the override values.
struct OverrideTransformer {
    pub target_id: String,
    pub field_path: String,
    pub override_key: ast::Identifier,
    pub override_value: ast::NodeRef<ast::Expr>,
    pub override_target_count: usize,
    pub has_override: bool,
    pub action: ast::OverrideAction,
}

impl<'ctx> MutSelfMutWalker<'ctx> for OverrideTransformer {
    fn walk_unification_stmt(&mut self, unification_stmt: &'ctx mut ast::UnificationStmt) {
        if unification_stmt.target.node.names[0] != self.target_id {
            return;
        }
        self.override_target_count = 1;
        self.has_override = true;
        self.walk_schema_expr(&mut unification_stmt.value.node);
    }

    fn walk_assign_stmt(&mut self, assign_stmt: &'ctx mut ast::AssignStmt) {
        if let ast::Expr::Schema(_) = &assign_stmt.value.node {
            self.override_target_count = 0;
            for target in &assign_stmt.targets {
                if target.node.names.len() != 1 {
                    continue;
                }
                if target.node.names[0] != self.target_id {
                    continue;
                }
                self.override_target_count += 1;
            }
            if self.override_target_count == 0 {
                return;
            }
            self.has_override = true;

            self.walk_expr(&mut assign_stmt.value.node);
        }
    }

    fn walk_schema_expr(&mut self, schema_expr: &'ctx mut ast::SchemaExpr) {
        if self.override_target_count == 0 {
            return;
        }
        if !self.find_schema_config_and_repalce(schema_expr) {
            // Not exist and append an override value when the action is CREATE_OR_UPDATE
            if let ast::OverrideAction::CreateOrUpdate = self.action {
                if let ast::Expr::Config(config_expr) = &mut schema_expr.config.node {
                    config_expr
                        .items
                        .push(Box::new(ast::Node::dummy_node(ast::ConfigEntry {
                            key: Some(Box::new(ast::Node::dummy_node(ast::Expr::Identifier(
                                self.override_key.clone(),
                            )))),
                            value: self.override_value.clone(),
                            operation: ast::ConfigEntryOperation::Override,
                            insert_index: -1,
                        })));
                }
            }
        }
        self.override_target_count = 0;
    }

    fn walk_config_expr(&mut self, config_expr: &'ctx mut ast::ConfigExpr) {
        for config_entry in config_expr.items.iter_mut() {
            walk_if_mut!(self, walk_expr, config_entry.node.key);
            self.walk_expr(&mut config_entry.node.value.node);
        }
    }

    fn walk_if_stmt(&mut self, _: &'ctx mut ast::IfStmt) {
        // Do not override AssignStmt in IfStmt
    }
    fn walk_schema_stmt(&mut self, _: &'ctx mut ast::SchemaStmt) {
        // Do not override AssignStmt in SchemaStmt
    }
    fn walk_lambda_expr(&mut self, _: &'ctx mut ast::LambdaExpr) {
        // Do not override AssignStmt in LambdaExpr
    }
}

impl OverrideTransformer {
    /// Get all field paths from AST nodes including schema and config.
    #[inline]
    pub(crate) fn get_field_paths(
        &mut self,
        expr: &mut ast::NodeRef<ast::Expr>,
    ) -> (Vec<String>, Vec<String>) {
        match &mut expr.node {
            ast::Expr::Schema(schema_expr) => self.get_schema_config_field_paths(schema_expr),
            ast::Expr::Config(config_expr) => self.get_config_field_paths(config_expr),
            _ => (vec![], vec![]),
        }
    }

    /// Get all field paths from a schema AST node.
    pub(crate) fn get_schema_config_field_paths(
        &mut self,
        schema_expr: &mut ast::SchemaExpr,
    ) -> (Vec<String>, Vec<String>) {
        if let ast::Expr::Config(config_expr) = &mut schema_expr.config.node {
            self.get_config_field_paths(config_expr)
        } else {
            (vec![], vec![])
        }
    }

    /// Get all field paths from a config AST node.
    pub(crate) fn get_config_field_paths(
        &mut self,
        config: &mut ast::ConfigExpr,
    ) -> (Vec<String>, Vec<String>) {
        let mut paths = vec![];
        let mut paths_with_id = vec![];
        for entry in config.items.iter_mut() {
            let (mut _paths, mut _paths_with_id) = self.get_entry_paths(&mut entry.node);
            paths.append(&mut _paths);
            paths_with_id.append(&mut _paths_with_id);
        }
        (paths, paths_with_id)
    }

    /// Get all field paths from a config entry.
    pub(crate) fn get_entry_paths(
        &mut self,
        entry: &mut ast::ConfigEntry,
    ) -> (Vec<String>, Vec<String>) {
        let mut paths = vec![];
        let mut paths_with_id = vec![];
        if let Some(key) = &entry.key {
            let path = match &key.node {
                ast::Expr::Identifier(identifier) => identifier.get_name(),
                ast::Expr::StringLit(string_lit) => string_lit.value.clone(),
                _ => return (paths, paths_with_id),
            };
            paths.push(path.clone());
            paths_with_id.push(path.clone());
            let (value_paths, value_paths_with_id) = self.get_field_paths(&mut entry.value);
            if !value_paths.is_empty() {
                paths.append(
                    &mut value_paths
                        .iter()
                        .map(|value_path| format!("{}.{}", path, value_path))
                        .collect::<Vec<String>>(),
                );
                paths_with_id.append(
                    &mut value_paths_with_id
                        .iter()
                        .map(|value_path| format!("{}|{}", path, value_path))
                        .collect::<Vec<String>>(),
                );
            }
        }
        (paths, paths_with_id)
    }

    /// Get config key path from the AST key node.
    #[inline]
    pub(crate) fn get_path_from_key(&mut self, key: &Option<ast::NodeRef<ast::Expr>>) -> String {
        match key {
            Some(key) => match &key.node {
                ast::Expr::Identifier(identifier) => identifier.get_name(),
                ast::Expr::StringLit(string_lit) => string_lit.value.clone(),
                _ => "".to_string(),
            },
            None => "".to_string(),
        }
    }
}

#[derive(Debug)]
enum OverrideConfig<'a> {
    Schema(&'a mut ast::SchemaExpr),
    Config(&'a mut ast::ConfigExpr),
}

impl OverrideTransformer {
    pub(crate) fn find_schema_config_and_repalce(
        &mut self,
        schema_expr: &mut ast::SchemaExpr,
    ) -> bool {
        let (paths, paths_with_id) = self.get_schema_config_field_paths(schema_expr);
        match paths.iter().position(|r| r == &self.field_path) {
            Some(pos) => {
                let mut config = OverrideConfig::Schema(schema_expr);
                self.replace_with_id_path(&mut config, &paths_with_id[pos]);
                true
            }
            None => false,
        }
    }

    pub(crate) fn replace_with_id_path(&mut self, config: &mut OverrideConfig, path_with_id: &str) {
        if path_with_id.is_empty() {
            return;
        }
        let parts = path_with_id.split('|').collect::<Vec<&str>>();
        for (i, part) in parts.iter().enumerate() {
            match config {
                OverrideConfig::Schema(schema_expr) => {
                    if let ast::Expr::Config(config_expr) = &mut schema_expr.config.node {
                        let mut delete_index_set = HashSet::new();
                        for (j, item) in config_expr.items.iter_mut().enumerate() {
                            let path = self.get_path_from_key(&item.node.key);
                            if &path == part {
                                match self.action {
                                    ast::OverrideAction::CreateOrUpdate => {
                                        if i == parts.len() - 1 {
                                            self.override_value.set_pos(item.pos());
                                            item.node.value = self.override_value.clone();
                                        }
                                        let path_with_id = &parts[i + 1..].join("|");
                                        match &mut item.node.value.node {
                                            ast::Expr::Schema(schema_expr) => {
                                                let mut config =
                                                    OverrideConfig::Schema(schema_expr);
                                                self.replace_with_id_path(
                                                    &mut config,
                                                    path_with_id,
                                                );
                                            }
                                            ast::Expr::Config(config_expr) => {
                                                let mut config =
                                                    OverrideConfig::Config(config_expr);
                                                self.replace_with_id_path(
                                                    &mut config,
                                                    path_with_id,
                                                );
                                            }
                                            _ => {}
                                        }
                                    }
                                    ast::OverrideAction::Delete => {
                                        delete_index_set.insert(j);
                                    }
                                }
                            }
                        }
                        if !delete_index_set.is_empty() {
                            let items: Vec<(usize, &ast::NodeRef<ast::ConfigEntry>)> = config_expr
                                .items
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| !delete_index_set.contains(i))
                                .collect();
                            config_expr.items = items
                                .iter()
                                .map(|(_, item)| {
                                    <&ast::NodeRef<ast::ConfigEntry>>::clone(item).clone()
                                })
                                .collect();
                        }
                    }
                }
                OverrideConfig::Config(config_expr) => {
                    let mut delete_index_set = HashSet::new();
                    for (j, item) in config_expr.items.iter_mut().enumerate() {
                        let path = self.get_path_from_key(&item.node.key);
                        if &path == part {
                            match self.action {
                                ast::OverrideAction::CreateOrUpdate => {
                                    if i == parts.len() - 1 && parts.len() == 1 {
                                        self.override_value.set_pos(item.pos());
                                        item.node.value = self.override_value.clone();
                                    }
                                    let path_with_id = &parts[i + 1..].join("|");
                                    match &mut item.node.value.node {
                                        ast::Expr::Schema(schema_expr) => {
                                            let mut config = OverrideConfig::Schema(schema_expr);
                                            self.replace_with_id_path(&mut config, path_with_id);
                                        }
                                        ast::Expr::Config(config_expr) => {
                                            let mut config = OverrideConfig::Config(config_expr);
                                            self.replace_with_id_path(&mut config, path_with_id);
                                        }
                                        _ => {}
                                    }
                                }
                                ast::OverrideAction::Delete => {
                                    delete_index_set.insert(j);
                                }
                            }
                        }
                    }
                    if !delete_index_set.is_empty() {
                        let items: Vec<(usize, &ast::NodeRef<ast::ConfigEntry>)> = config_expr
                            .items
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| !delete_index_set.contains(i))
                            .collect();
                        config_expr.items = items
                            .iter()
                            .map(|(_, item)| <&ast::NodeRef<ast::ConfigEntry>>::clone(item).clone())
                            .collect();
                    }
                }
            }
        }
    }
}
