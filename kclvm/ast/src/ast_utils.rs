use crate::ast::*;

#[macro_export]
macro_rules! node_ref {
    ($node: expr) => {
        NodeRef::new(Node::dummy_node($node))
    };
    ($node: expr, $pos:expr) => {
        NodeRef::new(Node::node_with_pos($node, $pos))
    };
}

#[macro_export]
macro_rules! expr_as {
    ($expr: expr, $expr_enum: path) => {
        if let $expr_enum(x) = ($expr.node as Expr) {
            Some(x)
        } else {
            None
        }
    };
}

#[macro_export]
macro_rules! stmt_as {
    ($stmt: expr, $stmt_enum: path) => {
        if let $stmt_enum(x) = ($stmt.node as Stmt) {
            Some(x)
        } else {
            None
        }
    };
}

pub struct AstUtil;
impl AstUtil {
    pub fn filter_schema_stmt(module: &NodeRef<Module>) -> Vec<NodeRef<SchemaStmt>> {
        let mut stmts = Vec::new();
        for stmt in &module.node.body {
            if let Stmt::Schema(schema_stmt) = &stmt.node {
                stmts.push(node_ref!(schema_stmt.clone()));
            }
        }
        return stmts;
    }

    pub fn build_assign_node(attr_name: Option<&str>, ast_node: NodeRef<Expr>) -> NodeRef<Stmt>{
        
        let attr_name = match attr_name{
            Some(a_name) => {a_name}
            None => {"value"}
        };

        let iden = node_ref!(Identifier{
            names:vec![attr_name.to_string()],
            pkgpath: String::new(),
            ctx: ExprContext::Store
        });

        node_ref!(Stmt::Assign(AssignStmt{
            value: ast_node,
            targets: vec![iden],
            type_annotation: None,
            ty: None
        }))
    }
}
