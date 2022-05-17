#![allow(dead_code)]

extern crate enquote;

use std::vec;

use super::int::bytes_to_int;
use super::Parser;

use either::{self, Either};
use kclvm_ast::node_ref;


use crate::parser::precedence::Precedence;
use kclvm_ast::ast::*;
use kclvm_ast::token;
use kclvm_ast::token::{BinOpToken, DelimToken, TokenKind};
use kclvm_span::symbol::kw;

/// Parser implementation of expressions, which consists of sub-expressions,
/// operand and tokens. Like the general LL1 paser, parser constantly looking for
/// left-side derivation, priority is specified by matching code explicitly.
/// The entrances of expression parsing are `parse_exprlist` and `parse_expr`.
/// TODO: operand design is quite complex, can be simplified later.
impl<'a> Parser<'a> {
    /// ~~~ Entrances

    /// Syntax:
    /// expr_list: expr (COMMA expr)*
    pub(crate) fn parse_exprlist(&mut self) -> Vec<NodeRef<Expr>> {
        let mut exprs = Vec::new();
        let expr = self.parse_expr();
        exprs.push(expr);

        loop {
            let token = self.token;
            match token.kind {
                TokenKind::Comma => {
                    self.bump();
                    let expr = self.parse_expr();
                    exprs.push(expr);
                }
                _ => break,
            }
        }

        exprs
    }

    /// Syntax:
    /// test: if_expr | simple_expr
    pub(crate) fn parse_expr(&mut self) -> NodeRef<Expr> {
        let operand = self.parse_simple_expr();

        // try if expr
        if self.token.is_keyword(kw::If) {
            return self.parse_if_expr(operand);
        }

        operand
    }

    /// Syntax:
    /// simple_expr: unary_expr | binary_expr | primary_expr
    /// unary_expr: un_op simple_expr
    /// binary_expr: simple_expr bin_op simple_expr
    pub(crate) fn parse_simple_expr(&mut self) -> NodeRef<Expr> {
        self.do_parse_simple_expr(Precedence::Lowest)
    }

    /// Syntax:
    /// identifier: NAME (DOT NAME)*
    pub(crate) fn parse_identifier_expr(&mut self) -> NodeRef<Expr> {
        let token = self.token;
        Box::new(Node::node(
            Expr::Identifier(self.parse_identifier().node),
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    fn do_parse_simple_expr(&mut self, prec1: Precedence) -> NodeRef<Expr> {
        let token = self.token;

        let mut x = self.parse_unary_expr();

        let mut cmp_expr = Compare {
            left: x.clone(),
            ops: Vec::new(),
            comparators: Vec::new(),
        };
        loop {
            // try bin expr
            // If current op in a op-right pair has a higher priority to prev one,
            // try to merging following tokens to binary exprs.
            // Otherwise, just return operand to merge to prev binary expr with no
            // operation affinity processing.

            let mut use_peek_op = false;
            let mut oprec = Precedence::from(self.token);

            if let Some(peek) = self.cursor.peek() {
                if self.token.is_keyword(kw::Not) && peek.is_keyword(kw::In) {
                    oprec = Precedence::InOrNotIn;
                    use_peek_op = true;
                }
                if self.token.is_keyword(kw::Is) && peek.is_keyword(kw::Not) {
                    oprec = Precedence::IsOrIsNot;
                    use_peek_op = true;
                }
            }

            if oprec <= prec1 {
                if !cmp_expr.ops.is_empty() {
                    return Box::new(Node::node(
                        Expr::Compare(cmp_expr),
                        self.sess.struct_token_loc(token, self.prev_token),
                    ));
                }
                return x;
            }

            let op = if use_peek_op {
                let peek = self.cursor.peek().unwrap();
                if self.token.is_keyword(kw::Not) && peek.is_keyword(kw::In) {
                    BinOrCmpOp::Cmp(CmpOp::NotIn)
                } else if self.token.is_keyword(kw::Is) && peek.is_keyword(kw::Not) {
                    BinOrCmpOp::Cmp(CmpOp::IsNot)
                } else {
                    panic!("unreachable")
                }
            } else {
                BinOrCmpOp::try_from(self.token)
                    .expect("invalid binary expr: missing binary operation")
            };

            self.bump();
            if use_peek_op {
                self.bump(); // bump peek
            }

            let y = self.do_parse_simple_expr(oprec);

            // compare: a == b == c
            if let BinOrCmpOp::Cmp(cmp_op) = op.clone() {
                if cmp_expr.ops.is_empty() {
                    cmp_expr.left = x.clone();
                }
                cmp_expr.ops.push(cmp_op);
                cmp_expr.comparators.push(y);
                continue;
            }

            if !cmp_expr.ops.is_empty() {
                x = Box::new(Node::node(
                    Expr::Compare(cmp_expr.clone()),
                    self.sess.struct_token_loc(token, self.prev_token),
                ));
                cmp_expr.ops = Vec::new();
                cmp_expr.comparators = Vec::new();
            }

            x = Box::new(Node::node(
                Expr::Binary(BinaryExpr {
                    left: x,
                    op,
                    right: y,
                }),
                self.sess.struct_token_loc(token, self.prev_token),
            ));
        }
    }

    /// ~~~ Sub Expressions

    /// Syntax:
    /// if_expr: simple_expr IF simple_expr ELSE test
    /// test: if_expr | simple_expr
    fn parse_if_expr(&mut self, body: NodeRef<Expr>) -> NodeRef<Expr> {
        let token = self.token;
        if self.token.is_keyword(kw::If) {
            self.bump();
            let cond = self.parse_simple_expr();
            if self.token.is_keyword(kw::Else) {
                self.bump();
                let orelse = self.parse_expr();
                Box::new(Node::node(
                    Expr::If(IfExpr { body, cond, orelse }),
                    self.sess.struct_token_loc(token, self.prev_token),
                ))
            } else {
                self.sess
                    .struct_token_error(&[&kw::Else.into()], self.token)
            }
        } else {
            self.sess.struct_token_error(&[&kw::If.into()], self.token)
        }
    }

    /// Syntax:
    /// primary_expr: operand | primary_expr select_suffix | primary_expr call_suffix | primary_expr slice_suffix
    /// Note: we need to look ahead 2 tokens to match select_suffix and slice_suffix, which actually breaks LL1 rule.
    fn parse_primary_expr(&mut self) -> NodeRef<Expr> {
        let mut operand = self.parse_operand_expr();

        loop {
            match self.token.kind {
                TokenKind::Dot => {
                    // select_suffix
                    operand = self.parse_selector_expr(operand)
                }
                TokenKind::Question => {
                    match self.cursor.peek() {
                        Some(token) => {
                            match token.kind {
                                TokenKind::Dot => {
                                    // select_suffix
                                    operand = self.parse_selector_expr(operand)
                                }
                                TokenKind::OpenDelim(DelimToken::Bracket) => {
                                    // slice_suffix
                                    operand = self.parse_subscript_expr(operand)
                                }
                                _ => break operand,
                            }
                        }
                        None => break operand,
                    }
                }
                TokenKind::OpenDelim(dt) => {
                    match dt {
                        DelimToken::Paren => {
                            // call_suffix
                            operand = self.parse_call_expr(operand)
                        }
                        DelimToken::Bracket => {
                            // slice_suffix
                            operand = self.parse_subscript_expr(operand)
                        }
                        _ => {}
                    }
                }
                _ => break operand,
            }
        }
    }

    /// Syntax:
    /// unary_expr: un_op simple_expr
    fn parse_unary_expr(&mut self) -> NodeRef<Expr> {
        let token = self.token;
        let op = if let Ok(x) = UnaryOp::try_from(self.token) {
            x
        } else {
            return self.parse_primary_expr();
        };

        self.bump();
        let operand = self.parse_primary_expr();

        Box::new(Node::node(
            Expr::Unary(UnaryExpr { op, operand }),
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    /// Syntax:
    /// select_suffix: [QUESTION] DOT NAME
    fn parse_selector_expr(&mut self, value: NodeRef<Expr>) -> NodeRef<Expr> {
        let token = self.token;
        let has_question = match self.token.kind {
            TokenKind::Question => {
                self.bump();
                true
            }
            _ => false,
        };
        // bump .
        self.bump();
        let attr = self.parse_identifier();
        Box::new(Node::node(
            Expr::Selector(SelectorExpr {
                value,
                attr,
                has_question,
                ctx: ExprContext::Load,
            }),
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    /// Syntax:
    /// call_suffix: LEFT_PARENTHESES [arguments [COMMA]] RIGHT_PARENTHESES
    fn parse_call_expr(&mut self, func: NodeRef<Expr>) -> NodeRef<Expr> {
        let token = self.token;
        let call_expr = self.parse_call(func);
        Box::new(Node::node(
            Expr::Call(call_expr),
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    fn parse_call(&mut self, func: NodeRef<Expr>) -> CallExpr {
        // LEFT_PARENTHESES
        match self.token.kind {
            TokenKind::OpenDelim(DelimToken::Paren) => self.bump(),
            _ => self.sess.struct_token_error(
                &[&TokenKind::OpenDelim(DelimToken::Paren).into()],
                self.token,
            ),
        }

        // arguments or empty
        let (args, keywords) = if self.token.kind == TokenKind::CloseDelim(DelimToken::Paren) {
            (Vec::new(), Vec::new())
        } else {
            self.parse_arguments_expr()
        };

        // [COMMA]
        if self.token.kind == TokenKind::Comma {
            self.bump()
        }

        // RIGHT_PARENTHESES
        match self.token.kind {
            TokenKind::CloseDelim(DelimToken::Paren) => self.bump(),
            _ => self.sess.struct_token_error(
                &[&TokenKind::CloseDelim(DelimToken::Paren).into()],
                self.token,
            ),
        }

        CallExpr {
            func,
            args,
            keywords,
        }
    }

    /// Syntax:
    /// slice_suffix: [QUESTION] LEFT_BRACKETS (expr | [expr] COLON [expr] [COLON [expr]]) RIGHT_BRACKETS
    fn parse_subscript_expr(&mut self, value: NodeRef<Expr>) -> NodeRef<Expr> {
        let token = self.token;
        let mut has_question = false;
        // [QUESTION]
        if self.token.kind == TokenKind::Question {
            self.bump();
            has_question = true;
        }

        // LEFT_BRACKETS
        match self.token.kind {
            TokenKind::OpenDelim(DelimToken::Bracket) => self.bump(),
            _ => self.sess.struct_token_error(
                &[&TokenKind::OpenDelim(DelimToken::Bracket).into()],
                self.token,
            ),
        }

        let mut round = 0;
        let mut is_slice = false;
        let mut colon_counter = 0;
        let mut exprs = vec![None, None, None];
        let mut expr_index = 0;
        let mut exprs_consecutive = 0;

        while round <= 4 {
            match self.token.kind {
                TokenKind::Colon => {
                    self.bump();
                    is_slice = true;
                    colon_counter += 1;
                    expr_index += 1;

                    if colon_counter > 2 {
                        self.sess
                            .struct_token_error(&[&"expression".to_string()], self.token)
                    }
                    exprs_consecutive -= 1
                }
                TokenKind::CloseDelim(DelimToken::Bracket) => break,
                _ => {
                    if !is_slice && round == 1 {
                        // it just has one round for an array
                        self.sess
                            .struct_compiler_bug("an list should have only one expr")
                    }

                    exprs[expr_index] = Some(self.parse_expr());
                    exprs_consecutive += 1;

                    if exprs_consecutive > 1 {
                        self.sess.struct_compiler_bug("consecutive exprs found.")
                    }
                }
            }
            round += 1;
        }

        if exprs.len() != 3 {
            self.sess
                .struct_compiler_bug("an slice should have three exprs.")
        }

        // RIGHT_BRACKETS
        match self.token.kind {
            TokenKind::CloseDelim(DelimToken::Bracket) => self.bump(),
            _ => self.sess.struct_token_error(
                &[&TokenKind::CloseDelim(DelimToken::Bracket).into()],
                self.token,
            ),
        }

        if is_slice {
            Box::new(Node::node(
                Expr::Subscript(Subscript {
                    value,
                    index: None,
                    lower: exprs[0].clone(),
                    upper: exprs[1].clone(),
                    step: exprs[2].clone(),
                    ctx: ExprContext::Load,
                    has_question,
                }),
                self.sess.struct_token_loc(token, self.prev_token),
            ))
        } else {
            if !(exprs[1].is_none() && exprs[2].is_none()) {
                self.sess
                    .struct_compiler_bug("an list should have only one expr.")
            }
            Box::new(Node::node(
                Expr::Subscript(Subscript {
                    value,
                    index: exprs[0].clone(),
                    lower: None,
                    upper: None,
                    step: None,
                    ctx: ExprContext::Load,
                    has_question,
                }),
                self.sess.struct_token_loc(token, self.prev_token),
            ))
        }
    }

    /// ~~~ Operand

    /// Syntax:
    /// operand: identifier | number | string | constant | quant_expr | list_expr | list_comp | config_expr | dict_comp | identifier call_suffix | schema_expr | lambda_expr | paren_expr
    fn parse_operand_expr(&mut self) -> NodeRef<Expr> {
        let token = self.token;
        // try primary expr
        match self.token.kind {
            TokenKind::Ident(_) => {
                // None
                if self.token.is_keyword(kw::None) {
                    return self.parse_constant_expr(token::None);
                }
                // Undefined
                if self.token.is_keyword(kw::Undefined) {
                    return self.parse_constant_expr(token::Undefined);
                }
                // Bool: True/False
                if self.token.is_keyword(kw::True) || self.token.is_keyword(kw::False) {
                    return self.parse_constant_expr(token::Bool);
                }

                // lambda expression
                if self.token.is_keyword(kw::Lambda) {
                    self.parse_lambda_expr()
                // quant expression
                } else if self.token.is_keyword(kw::Any)
                    || self.token.is_keyword(kw::All)
                    || self.token.is_keyword(kw::Map)
                    || self.token.is_keyword(kw::Filter)
                {
                    self.parse_quant_expr()
                } else {
                    // identifier
                    let mut operand = self.parse_identifier_expr();

                    // identifier call_suffix | schema_expr
                    match self.token.kind {
                        TokenKind::OpenDelim(DelimToken::Brace) => {
                            // schema expression without args
                            operand = self.parse_schema_expr(*operand, token)
                        }
                        TokenKind::OpenDelim(DelimToken::Paren) => {
                            let call = self.parse_call(Box::new(*operand));

                            if let TokenKind::OpenDelim(DelimToken::Brace) = self.token.kind {
                                // schema expression with args
                                operand = self.parse_schema_expr_with_args(call, token)
                            } else {
                                // identifier call_suffix
                                return Box::new(Node::node(
                                    Expr::Call(call),
                                    self.sess.struct_token_loc(token, self.prev_token),
                                ));
                            }
                        }
                        _ => (),
                    }

                    operand
                }
            }
            TokenKind::Literal(lk) => {
                // lit expr
                match lk.kind {
                    token::LitKind::Bool => self.parse_constant_expr(lk.kind),
                    token::LitKind::Integer | token::LitKind::Float => self.parse_num_expr(lk),
                    token::LitKind::Str { .. } => self.parse_str_expr(lk),
                    // Note: None and Undefined are handled in ident, skip handle them here.
                    _ => self.sess.struct_token_error(
                        &[
                            &token::LitKind::Bool.into(),
                            &token::LitKind::Integer.into(),
                            &token::LitKind::Str {
                                is_long_string: false,
                                is_raw: false,
                            }
                            .into(),
                        ],
                        self.token,
                    ),
                }
            }
            TokenKind::OpenDelim(dt) => {
                // list expr, dict expr, paren expr
                match dt {
                    // paren expr
                    DelimToken::Paren => self.parse_paren_expr(),
                    // list expr or list comp
                    DelimToken::Bracket => self.parse_list_expr(),
                    // dict expr or dict comp
                    DelimToken::Brace => self.parse_config_expr(),
                    _ => self.sess.struct_token_error(
                        &[
                            &TokenKind::OpenDelim(DelimToken::Paren).into(),
                            &TokenKind::OpenDelim(DelimToken::Bracket).into(),
                            &TokenKind::OpenDelim(DelimToken::Brace).into(),
                        ],
                        self.token,
                    ),
                }
            }
            _ => self.sess.struct_token_error(
                &[
                    &TokenKind::ident_value(),
                    &TokenKind::literal_value(),
                    &TokenKind::OpenDelim(DelimToken::NoDelim).into(),
                ],
                self.token,
            ),
        }
    }

    fn match_operand_expr(&self) -> bool {
        matches!(
            self.token.kind,
            TokenKind::Literal(_) | TokenKind::Ident(_) | TokenKind::OpenDelim(_)
        )
    }

    /// Syntax:
    /// quant_expr: quant_op [ identifier COMMA ] identifier IN quant_target LEFT_BRACE (expr [IF expr]
    /// | NEWLINE _INDENT simple_expr [IF expr] NEWLINE _DEDENT)? RIGHT_BRACE
    /// quant_target: string | identifier | list_expr | list_comp | dict_expr | dict_comp
    /// quant_op: ALL | ANY | FILTER | MAP
    fn parse_quant_expr(&mut self) -> NodeRef<Expr> {
        let token = self.token;
        // quant_op
        let op = if self.token.is_keyword(kw::All) {
            QuantOperation::All
        } else if self.token.is_keyword(kw::Any) {
            QuantOperation::Any
        } else if self.token.is_keyword(kw::Filter) {
            QuantOperation::Filter
        } else if self.token.is_keyword(kw::Map) {
            QuantOperation::Map
        } else {
            self.sess.struct_token_error(
                &[
                    &QuantOperation::All.into(),
                    &QuantOperation::Any.into(),
                    &QuantOperation::Filter.into(),
                    &QuantOperation::Map.into(),
                ],
                self.token,
            )
        };
        self.bump();

        // [ identifier COMMA ] identifier
        let mut variables = vec![self.parse_identifier()];
        if self.token.kind == TokenKind::Comma {
            self.bump();
            variables.push(self.parse_identifier());
        }

        // IN
        if self.token.is_keyword(kw::In) {
            self.bump();
        } else {
            self.sess.struct_token_error(&[&kw::In.into()], self.token)
        }

        // quant_target
        let target = self.parse_quant_target_expr();

        // LEFT_BRACE
        match self.token.kind {
            TokenKind::OpenDelim(DelimToken::Brace) => {
                self.bump();
            }
            _ => self.sess.struct_token_error(
                &[&TokenKind::OpenDelim(DelimToken::Brace).into()],
                self.token,
            ),
        }

        // NEWLINE _INDENT
        let has_newline = if self.token.kind == TokenKind::Newline {
            self.skip_newlines();

            if self.token.kind == TokenKind::Indent {
                self.bump();
            } else {
                self.sess
                    .struct_token_error(&[&TokenKind::Indent.into()], self.token)
            }

            true
        } else {
            false
        };

        // expr
        let test = self.parse_simple_expr();

        // [IF epxr]
        let if_cond = if self.token.is_keyword(kw::If) {
            self.bump();

            Some(self.parse_expr())
        } else {
            None
        };

        // NEWLINE _DEDENT
        if has_newline {
            if self.token.kind == TokenKind::Newline {
                self.skip_newlines();
            } else {
                self.sess
                    .struct_token_error(&[&TokenKind::Newline.into()], self.token)
            }

            if self.token.kind == TokenKind::Dedent {
                self.bump();
            } else {
                self.sess
                    .struct_token_error(&[&TokenKind::Dedent.into()], self.token)
            }
        }

        // RIGHT_BRACE
        match self.token.kind {
            TokenKind::CloseDelim(DelimToken::Brace) => {
                self.bump();
            }
            _ => self.sess.struct_token_error(
                &[&TokenKind::CloseDelim(DelimToken::Brace).into()],
                self.token,
            ),
        }

        Box::new(Node::node(
            Expr::Quant(QuantExpr {
                target,
                variables,
                op,
                test,
                if_cond,
                ctx: ExprContext::Load,
            }),
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    /// Syntax:
    /// quant_target: string | identifier | list_expr | list_comp | dict_expr | dict_comp
    fn parse_quant_target_expr(&mut self) -> NodeRef<Expr> {
        // try primary expr
        match self.token.kind {
            TokenKind::Ident(_) => {
                if self.token.is_keyword(kw::None)
                    | self.token.is_keyword(kw::Undefined)
                    | self.token.is_keyword(kw::Lambda)
                    | self.token.is_keyword(kw::Any)
                    || self.token.is_keyword(kw::All)
                    || self.token.is_keyword(kw::Map)
                    || self.token.is_keyword(kw::Filter)
                {
                    self.sess.struct_token_error(
                        &[
                            &kw::None.into(),
                            &kw::Undefined.into(),
                            &kw::Lambda.into(),
                            &kw::Any.into(),
                            &kw::All.into(),
                            &kw::Map.into(),
                            &kw::Filter.into(),
                        ],
                        self.token,
                    )
                } else {
                    // identifier
                    self.parse_identifier_expr()
                }
            }
            TokenKind::Literal(lk) => {
                // lit expr
                match lk.kind {
                    token::LitKind::Str { .. } => self.parse_str_expr(lk),
                    // Note: None and Undefined are handled in ident, skip handle them here.
                    _ => self.sess.struct_token_error(
                        &[&token::LitKind::Str {
                            is_long_string: false,
                            is_raw: false,
                        }
                        .into()],
                        self.token,
                    ),
                }
            }
            TokenKind::OpenDelim(dt) => {
                // list expr, dict expr, paren expr
                match dt {
                    // list expr or list comp
                    DelimToken::Bracket => self.parse_list_expr(),
                    // dict expr or dict comp
                    DelimToken::Brace => self.parse_config_expr(),
                    _ => self.sess.struct_token_error(
                        &[
                            &TokenKind::OpenDelim(DelimToken::Bracket).into(),
                            &TokenKind::OpenDelim(DelimToken::Brace).into(),
                        ],
                        self.token,
                    ),
                }
            }
            _ => self.sess.struct_token_error(
                &[
                    &TokenKind::ident_value(),
                    &TokenKind::literal_value(),
                    &TokenKind::OpenDelim(DelimToken::NoDelim).into(),
                ],
                self.token,
            ),
        }
    }

    /// Syntax:
    /// list_expr: LEFT_BRACKETS [list_items | NEWLINE _INDENT list_items _DEDENT] RIGHT_BRACKETS
    /// list_comp: LEFT_BRACKETS (expr comp_clause+ | NEWLINE _INDENT expr comp_clause+ _DEDENT) RIGHT_BRACKETS
    fn parse_list_expr(&mut self) -> NodeRef<Expr> {
        let token = self.token;
        // LEFT_BRACKETS
        self.bump();

        // try RIGHT_BRACKETS: empty config
        if let TokenKind::CloseDelim(DelimToken::Bracket) = self.token.kind {
            self.bump();

            return Box::new(Node::node(
                Expr::List(ListExpr {
                    elts: vec![],
                    ctx: ExprContext::Load,
                }),
                self.sess.struct_token_loc(token, self.prev_token),
            ));
        }

        let has_newline = if self.token.kind == TokenKind::Newline {
            self.skip_newlines();
            if self.token.kind == TokenKind::Indent {
                self.bump();
            } else if self.token.kind == TokenKind::CloseDelim(DelimToken::Bracket) {
                self.bump();
                return Box::new(Node::node(
                    Expr::List(ListExpr {
                        elts: vec![],
                        ctx: ExprContext::Load,
                    }),
                    self.sess.struct_token_loc(token, self.prev_token),
                ));
            } else {
                self.sess
                    .struct_token_error(&[&TokenKind::Indent.into()], self.token)
            }
            true
        } else {
            false
        };

        let items = self.parse_list_items();
        let generators = self.parse_comp_clauses();

        // _DEDENT
        if has_newline {
            self.skip_newlines();
            if self.token.kind == TokenKind::Dedent {
                self.bump();
            } else {
                self.sess
                    .struct_token_error(&[&TokenKind::Dedent.into()], self.token)
            }
        }

        // RIGHT_BRACKETS
        match self.token.kind {
            TokenKind::CloseDelim(DelimToken::Bracket) => {
                self.bump();
            }
            _ => self.sess.struct_token_error(
                &[&TokenKind::CloseDelim(DelimToken::Bracket).into()],
                self.token,
            ),
        }

        if !generators.is_empty() {
            if items.len() > 1 {
                self.sess.struct_compiler_bug("multiple items found.")
            }

            Box::new(Node::node(
                Expr::ListComp(ListComp {
                    elt: items[0].clone(),
                    generators,
                }),
                self.sess.struct_token_loc(token, self.prev_token),
            ))
        } else {
            Box::new(Node::node(
                Expr::List(ListExpr {
                    elts: items,
                    ctx: ExprContext::Load,
                }),
                self.sess.struct_token_loc(token, self.prev_token),
            ))
        }
    }

    /// Syntax:
    /// list_items: expr ((COMMA [NEWLINE] | NEWLINE) expr)* [COMMA] [NEWLINE]
    pub(crate) fn parse_list_items(&mut self) -> Vec<NodeRef<Expr>> {
        if let TokenKind::CloseDelim(DelimToken::Bracket) = self.token.kind {
            return Vec::new();
        }

        let mut items = vec![self.parse_list_item()];
        if let TokenKind::Comma = self.token.kind {
            self.bump();
        }
        self.skip_newlines();

        loop {
            if matches!(
                self.token.kind,
                TokenKind::CloseDelim(DelimToken::Bracket) | TokenKind::Dedent
            ) {
                break;
            }
            if self.token.is_keyword(kw::For) {
                break;
            }

            if let TokenKind::Comma = self.token.kind {
                self.bump();
            }
            self.skip_newlines();

            items.push(self.parse_list_item());
            if let TokenKind::Comma = self.token.kind {
                self.bump();
            }
            self.skip_newlines();
        }

        items
    }

    /// Syntax:
    /// list_item: test | star_expr | if_item
    fn parse_list_item(&mut self) -> NodeRef<Expr> {
        let token = self.token;
        let item;

        match self.token.kind {
            TokenKind::BinOp(BinOpToken::Star) => {
                self.bump();
                let expr = self.parse_expr();
                let pos = self.token_span_pos(token, self.prev_token);
                item = node_ref!(
                    Expr::Starred(StarredExpr {
                        value: expr,
                        ctx: ExprContext::Load,
                    }),
                    pos
                );
            }
            _ => {
                if self.token.is_keyword(kw::If) {
                    item = self.parse_if_item_expr();
                } else {
                    item = self.parse_expr();
                }
            }
        }

        item
    }

    /// Syntax:
    /// if_item:
    ///   IF test COLON if_item_exec_block
    ///   (ELIF test COLON if_item_exec_block)*
    ///   (ELSE COLON if_item_exec_block)?
    fn parse_if_item_expr(&mut self) -> NodeRef<Expr> {
        let token = self.token;
        let mut need_skip_newlines = false;

        let mut if_item = {
            self.bump_keyword(kw::If);

            let if_cond = self.parse_expr();
            self.bump_token(TokenKind::Colon);

            if let TokenKind::Newline = self.token.kind {
                need_skip_newlines = true;
            }

            let exprs = self.parse_if_item_exec_block(need_skip_newlines);

            ListIfItemExpr {
                if_cond,
                exprs,
                orelse: None,
            }
        };

        if let TokenKind::Newline = self.token.kind {
            self.skip_newlines();
        }

        // elif ...
        let mut elif_list = Vec::new();
        loop {
            if !self.token.is_keyword(kw::Elif) {
                break;
            }

            let token = self.token;
            self.bump_keyword(kw::Elif);

            let elif_cond = self.parse_expr();
            self.bump_token(TokenKind::Colon);

            if let TokenKind::Newline = self.token.kind {
                need_skip_newlines = true;
            }

            let exprs = self.parse_if_item_exec_block(need_skip_newlines);
            let x = ListIfItemExpr {
                if_cond: elif_cond,
                exprs,
                orelse: None,
            };

            elif_list.push(Box::new(Node::node(
                x,
                self.sess.struct_token_loc(token, self.prev_token),
            )));
        }

        if let TokenKind::Newline = self.token.kind {
            self.skip_newlines();
        }

        // else
        if self.token.is_keyword(kw::Else) {
            let token = self.token;

            self.bump_keyword(kw::Else);
            self.bump_token(TokenKind::Colon);

            if let TokenKind::Newline = self.token.kind {
                need_skip_newlines = true;
            }

            let else_body = self.parse_if_item_exec_block(need_skip_newlines);

            let t = Box::new(Node::node(
                Expr::List(ListExpr {
                    elts: else_body,
                    ctx: ExprContext::Load,
                }),
                self.sess.struct_token_loc(token, self.prev_token),
            ));

            if_item.orelse = Some(t);
        }

        // fix elif-list and else
        while let Some(mut x) = elif_list.pop() {
            x.node.orelse = if_item.orelse;

            let t = Node {
                node: Expr::ListIfItem(x.node),
                filename: x.filename,
                line: x.line,
                column: x.column,
                end_line: x.end_line,
                end_column: x.end_column,
            };

            if_item.orelse = Some(Box::new(t));
        }

        Box::new(Node::node(
            Expr::ListIfItem(if_item),
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    /// Syntax:
    /// if_item_exec_block
    ///     : list_item [NEWLINE]
    ///     | NEWLINE _INDENT
    ///         list_item ((COMMA [NEWLINE] | NEWLINE) list_item)*
    ///         [COMMA] [NEWLINE]
    ///     _DEDENT
    fn parse_if_item_exec_block(&mut self, need_skip_newlines: bool) -> Vec<NodeRef<Expr>> {
        if !need_skip_newlines {
            return vec![self.parse_list_item()];
        }

        self.skip_newlines();
        self.bump_token(TokenKind::Indent);

        let mut body = Vec::new();

        loop {
            if matches!(self.token.kind, TokenKind::Dedent) {
                break;
            }

            body.push(self.parse_list_item());

            if matches!(self.token.kind, TokenKind::Comma) {
                self.bump();
            }
            self.skip_newlines();
        }

        self.bump_token(TokenKind::Dedent);
        body
    }

    /// Syntax:
    /// config_expr: LEFT_BRACE [config_entries | NEWLINE _INDENT config_entries _DEDENT] RIGHT_BRACE
    /// config_entries: config_entry ((COMMA [NEWLINE] | NEWLINE) config_entry)* [COMMA] [NEWLINE]
    /// config_comp: LEFT_BRACE (config_entry comp_clause+ | NEWLINE _INDENT config_entry comp_clause+ _DEDENT) RIGHT_BRACE
    fn parse_config_expr(&mut self) -> NodeRef<Expr> {
        let token = self.token;
        // LEFT_BRACE
        self.bump();

        // try RIGHT_BRACE: empty config
        if let TokenKind::CloseDelim(DelimToken::Brace) = self.token.kind {
            self.bump();

            return Box::new(Node::node(
                Expr::Config(ConfigExpr { items: vec![] }),
                self.sess.struct_token_loc(token, self.prev_token),
            ));
        }

        let has_newline = if self.token.kind == TokenKind::Newline {
            self.skip_newlines();
            if self.token.kind == TokenKind::Indent {
                self.bump();
            } else if self.token.kind == TokenKind::CloseDelim(DelimToken::Brace) {
                self.bump();
                return Box::new(Node::node(
                    Expr::Config(ConfigExpr { items: vec![] }),
                    self.sess.struct_token_loc(token, self.prev_token),
                ));
            } else {
                self.sess
                    .struct_token_error(&[&TokenKind::Indent.into()], self.token)
            }
            true
        } else {
            false
        };

        let items = self.parse_config_entries();
        let generators = self.parse_comp_clauses();

        // _DEDENT
        if has_newline {
            self.skip_newlines();
            if self.token.kind == TokenKind::Dedent {
                self.bump();
            } else {
                self.sess
                    .struct_token_error(&[&TokenKind::Dedent.into()], self.token)
            }
        }

        // RIGHT_BRACE
        match self.token.kind {
            TokenKind::CloseDelim(DelimToken::Brace) => {
                self.bump();
            }
            _ => self.sess.struct_token_error(
                &[&TokenKind::CloseDelim(DelimToken::Brace).into()],
                self.token,
            ),
        }

        if !generators.is_empty() {
            if items.len() > 1 {
                self.sess.struct_compiler_bug("multiple entries found.")
            }

            Box::new(Node::node(
                Expr::DictComp(DictComp {
                    entry: items[0].node.clone(),
                    generators,
                }),
                self.sess.struct_token_loc(token, self.prev_token),
            ))
        } else {
            Box::new(Node::node(
                Expr::Config(ConfigExpr { items }),
                self.sess.struct_token_loc(token, self.prev_token),
            ))
        }
    }

    /// Syntax:
    /// config_entries: config_entry ((COMMA [NEWLINE] | NEWLINE) config_entry)* [COMMA] [NEWLINE]
    fn parse_config_entries(&mut self) -> Vec<NodeRef<ConfigEntry>> {
        let mut entries = vec![self.parse_config_entry()];
        if let TokenKind::Comma = self.token.kind {
            self.bump();
        }
        self.skip_newlines();

        loop {
            if matches!(
                self.token.kind,
                TokenKind::CloseDelim(DelimToken::Brace) | TokenKind::Dedent
            ) {
                break;
            }
            if self.token.is_keyword(kw::For) {
                break;
            }

            if let TokenKind::Comma = self.token.kind {
                self.bump();
            }
            self.skip_newlines();

            entries.push(self.parse_config_entry());

            if let TokenKind::Comma = self.token.kind {
                self.bump();
            }
            self.skip_newlines();
        }

        entries
    }

    /// Syntax:
    /// config_entry: expr (COLON | ASSIGN | COMP_PLUS) expr | double_star_expr | if_entry
    /// entry: expr (COLON | ASSIGN | COMP_PLUS) expr | double_star_expr
    /// Note: use the same ast node here for simplicity, do semantic checking later
    fn parse_config_entry(&mut self) -> NodeRef<ConfigEntry> {
        let token = self.token;
        let key;
        let value;
        let operation;

        match self.token.kind {
            TokenKind::BinOp(BinOpToken::StarStar) => {
                self.bump();
                key = None;
                value = self.parse_expr();
                operation = ConfigEntryOperation::Union;
            }
            _ => {
                if self.token.is_keyword(kw::If) {
                    key = None;
                    value = self.parse_if_entry_expr();
                    operation = ConfigEntryOperation::Union;
                } else {
                    key = Some(self.parse_expr());
                    match self.token.kind {
                        TokenKind::Colon => {
                            operation = ConfigEntryOperation::Union;
                        }
                        TokenKind::Assign => {
                            operation = ConfigEntryOperation::Override;
                        }
                        TokenKind::BinOpEq(BinOpToken::Plus) => {
                            operation = ConfigEntryOperation::Insert;
                        }
                        _ => self.sess.struct_token_error(
                            &[
                                &TokenKind::Colon.into(),
                                &TokenKind::Assign.into(),
                                &TokenKind::BinOpEq(BinOpToken::Plus).into(),
                            ],
                            self.token,
                        ),
                    }
                    self.bump();
                    value = self.parse_expr();
                }
            }
        }

        Box::new(Node::node(
            ConfigEntry {
                key,
                value,
                operation,
                insert_index: -1,
            },
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    /// Syntax:
    /// comp_clause+
    fn parse_comp_clauses(&mut self) -> Vec<NodeRef<CompClause>> {
        let mut clauses = Vec::new();

        loop {
            // comp_clause+
            if self.token.is_keyword(kw::For) {
                let clause = self.parse_comp_clause();
                clauses.push(clause);
            } else {
                break;
            }
        }

        clauses
    }

    /// Syntax:
    /// comp_clause: FOR loop_variables [COMMA] IN simple_expr [NEWLINE] (IF expr)*
    /// loop_variables: identifier (COMMA identifier)*
    fn parse_comp_clause(&mut self) -> NodeRef<CompClause> {
        let token = self.token;
        self.bump();

        let mut targets = vec![self.parse_identifier()];

        while let TokenKind::Comma = self.token.kind {
            self.bump();
            targets.push(self.parse_identifier());
        }

        // [COMMA]
        if self.token.kind == TokenKind::Comma {
            self.bump();
        }

        if !self.token.is_keyword(kw::In) {
            self.sess.struct_token_error(&[&kw::In.into()], self.token)
        }
        self.bump();

        let iter = self.parse_simple_expr();

        // [NEWLINE]
        if self.token.kind == TokenKind::Newline {
            self.skip_newlines();
        }

        let mut ifs = Vec::new();

        // (IF expr)*
        loop {
            if self.token.is_keyword(kw::If) {
                self.bump();

                ifs.push(self.parse_expr());
            } else {
                break;
            }
        }

        Box::new(Node::node(
            CompClause { targets, iter, ifs },
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    /// Syntax:
    /// if_entry:
    ///   IF expr COLON if_entry_exec_block
    ///   (ELIF expr COLON if_entry_exec_block)*
    ///   (ELSE COLON if_entry_exec_block)?
    fn parse_if_entry_expr(&mut self) -> NodeRef<Expr> {
        let mut need_skip_newlines = false;

        let mut if_entry = {
            self.bump_keyword(kw::If);

            let if_cond = self.parse_expr();
            self.bump_token(TokenKind::Colon);

            if let TokenKind::Newline = self.token.kind {
                need_skip_newlines = true;
            }

            let mut body = self.parse_if_entry_exec_block(need_skip_newlines);
            body.node.if_cond = if_cond;
            body
        };

        if let TokenKind::Newline = self.token.kind {
            self.skip_newlines();
        }

        // elif ...
        let mut need_skip_newlines = false;
        let mut elif_list = Vec::new();
        loop {
            if !self.token.is_keyword(kw::Elif) {
                break;
            }

            let token = self.token;
            self.bump_keyword(kw::Elif);

            let elif_cond = self.parse_expr();
            self.bump_token(TokenKind::Colon);

            if let TokenKind::Newline = self.token.kind {
                need_skip_newlines = true;
            }

            let elif_body = self.parse_if_entry_exec_block(need_skip_newlines);
            let x = ConfigIfEntryExpr {
                if_cond: elif_cond,
                items: elif_body.node.items,
                orelse: None,
            };

            elif_list.push(Box::new(Node::node(
                x,
                self.sess.struct_token_loc(token, self.prev_token),
            )));
        }

        if let TokenKind::Newline = self.token.kind {
            self.skip_newlines();
        }

        // else
        let mut need_skip_newlines = false;
        if self.token.is_keyword(kw::Else) {
            let token = self.token;

            self.bump_keyword(kw::Else);
            self.bump_token(TokenKind::Colon);

            if let TokenKind::Newline = self.token.kind {
                need_skip_newlines = true;
            }

            let else_body = self.parse_if_entry_exec_block(need_skip_newlines);

            let mut orelse = ConfigExpr { items: Vec::new() };
            for item in else_body.node.items {
                orelse.items.push(item);
            }

            let t = Box::new(Node::node(
                Expr::Config(orelse),
                self.sess.struct_token_loc(token, self.prev_token),
            ));

            if_entry.node.orelse = Some(t);
        }

        if let TokenKind::Comma = self.token.kind {
            self.bump();
        }
        if let TokenKind::Newline = self.token.kind {
            self.skip_newlines();
        }

        // fix elif-list and else
        while let Some(mut x) = elif_list.pop() {
            x.node.orelse = if_entry.node.orelse;

            let t = Node {
                node: Expr::ConfigIfEntry(x.node),
                filename: x.filename,
                line: x.line,
                column: x.column,
                end_line: x.end_line,
                end_column: x.end_column,
            };

            if_entry.node.orelse = Some(Box::new(t));
        }
        Box::new(Node::new(
            Expr::ConfigIfEntry(if_entry.node),
            if_entry.filename,
            if_entry.line,
            if_entry.column,
            if_entry.end_line,
            if_entry.end_column,
        ))
    }

    /// Syntax:
    /// if_entry_exec_block:
    /// if_entry_exec_block
    ///     : (test (COLON | ASSIGN | COMP_PLUS) test | double_star_expr | if_entry) [NEWLINE]
    ///     | NEWLINE _INDENT
    ///         (test (COLON | ASSIGN | COMP_PLUS) test | double_star_expr | if_entry)
    ///     ((COMMA [NEWLINE] | [NEWLINE])
    ///         (test (COLON | ASSIGN | COMP_PLUS) test | double_star_expr | if_entry))* [COMMA] [NEWLINE]
    ///     _DEDENT
    fn parse_if_entry_exec_block(
        &mut self,
        need_skip_newlines: bool,
    ) -> NodeRef<ConfigIfEntryExpr> {
        if need_skip_newlines {
            self.skip_newlines();
            self.bump_token(TokenKind::Indent);
        }

        let token = self.token;

        let mut body = {
            let node = Node {
                node: Expr::NameConstantLit(NameConstantLit {
                    value: NameConstant::None, // ignore
                }),
                filename: "".to_string(),
                line: 0,
                column: 0,
                end_line: 0,
                end_column: 0,
            };
            ConfigIfEntryExpr {
                if_cond: Box::new(node),
                items: vec![],
                orelse: None,
            }
        };

        fn parse_body_item(
            this: &mut Parser,
            body: &mut ConfigIfEntryExpr,
            need_skip_newlines: bool,
        ) -> bool {
            if need_skip_newlines {
                if let TokenKind::Dedent = this.token.kind {
                    return false;
                }
            } else if let TokenKind::Newline = this.token.kind {
                return false;
            }

            if this.token.is_keyword(kw::Elif) || this.token.is_keyword(kw::Else) {
                return false;
            }

            // if_entry
            if this.token.is_keyword(kw::If) {
                let expr0 = None;
                let expr1 = this.parse_if_entry_expr();
                let op = ConfigEntryOperation::Override;
                let pos = expr1.pos();
                body.items.push(node_ref!(
                    ConfigEntry {
                        key: expr0,
                        value: expr1,
                        operation: op,
                        insert_index: -1
                    },
                    pos
                ));

                return true;
            }

            if let TokenKind::Dedent = this.token.kind {
                return false;
            }
            if let TokenKind::Newline = this.token.kind {
                return false;
            }

            // double_star_expr
            if let TokenKind::BinOp(BinOpToken::StarStar) = this.token.kind {
                this.bump();

                let expr0 = None;
                let expr1 = this.parse_primary_expr();
                let op = ConfigEntryOperation::Override;

                let pos = expr1.pos();

                body.items.push(node_ref!(
                    ConfigEntry {
                        key: expr0,
                        value: expr1,
                        operation: op,
                        insert_index: -1
                    },
                    pos
                ));

                return true;
            }

            if let TokenKind::Dedent = this.token.kind {
                return false;
            }
            if let TokenKind::Newline = this.token.kind {
                return false;
            }

            // expr (COLON | ASSIGN | COMP_PLUS) expr
            {
                let expr0 = Some(this.parse_expr());
                let op = match this.token.kind {
                    TokenKind::Colon => {
                        this.bump();
                        ConfigEntryOperation::Union
                    }
                    TokenKind::Assign => {
                        this.bump();
                        ConfigEntryOperation::Override
                    }
                    TokenKind::BinOpEq(BinOpToken::Plus) => {
                        this.bump();
                        ConfigEntryOperation::Insert
                    }
                    _ => {
                        panic!("invalid op: {:?}", this.token);
                    }
                };

                let expr1 = this.parse_expr();

                let pos = expr1.pos();

                body.items.push(node_ref!(
                    ConfigEntry {
                        key: expr0,
                        value: expr1,
                        operation: op,
                        insert_index: -1
                    },
                    pos
                ));
            }

            true
        }

        while parse_body_item(self, &mut body, need_skip_newlines) {
            if let TokenKind::Comma = self.token.kind {
                self.bump();
            }
            if need_skip_newlines {
                self.skip_newlines();
            }
        }
        if let TokenKind::Newline = self.token.kind {
            self.bump();
        }

        if need_skip_newlines {
            self.skip_newlines();
            self.bump_token(TokenKind::Dedent);

            Box::new(Node::node(
                body,
                self.sess.struct_token_loc(token, self.prev_token),
            ))
        } else {
            Box::new(Node::node(
                body,
                self.sess.struct_token_loc(token, self.prev_token),
            ))
        }
    }

    /// Syntax:
    /// schema_expr: identifier config_expr
    pub(crate) fn parse_schema_expr(
        &mut self,
        identifier: Node<Expr>,
        lo: token::Token,
    ) -> NodeRef<Expr> {
        let result = identifier.try_into();

        let name = match result {
            Ok(v) => v,
            Err(_) => self
                .sess
                .struct_token_error(&[&TokenKind::ident_value()], self.token),
        };

        // config_expr
        let config = self.parse_config_expr();
        Box::new(Node::node(
            Expr::Schema(SchemaExpr {
                name: Box::new(name),
                args: Vec::new(),
                kwargs: Vec::new(),
                config,
            }),
            self.sess.struct_token_loc(lo, self.prev_token),
        ))
    }

    /// Syntax:
    /// schema_expr: identifier LEFT_PARENTHESES [arguments] RIGHT_PARENTHESES config_expr
    fn parse_schema_expr_with_args(&mut self, call: CallExpr, lo: token::Token) -> NodeRef<Expr> {
        let result = call.func.as_ref().clone().try_into();

        let name = match result {
            Ok(v) => v,
            Err(_) => self
                .sess
                .struct_token_error(&[&TokenKind::ident_value()], self.token),
        };

        // config_expr
        let config = self.parse_config_expr();
        Box::new(Node::node(
            Expr::Schema(SchemaExpr {
                name: Box::new(name),
                args: call.args,
                kwargs: call.keywords,
                config,
            }),
            self.sess.struct_token_loc(lo, self.prev_token),
        ))
    }

    /// Syntax:
    /// lambda_expr: LAMBDA [arguments] [RIGHT_ARROW type]
    ///   LEFT_BRACE
    ///     [expr_stmt | NEWLINE _INDENT schema_init_stmt+ _DEDENT]
    ///   RIGHT_BRACE
    fn parse_lambda_expr(&mut self) -> NodeRef<Expr> {
        let token = self.token;
        self.bump_keyword(kw::Lambda);

        let mut args = None;
        let mut return_type_str = None;
        let mut return_ty = None;

        // schema_arguments
        if !matches!(self.token.kind, TokenKind::RArrow | TokenKind::OpenDelim(_)) {
            args = self.parse_parameters(
                &[],
                &[TokenKind::RArrow, TokenKind::OpenDelim(DelimToken::Brace)],
                false,
            );
        }

        // return type
        if let TokenKind::RArrow = self.token.kind {
            self.bump_token(TokenKind::RArrow);
            let typ = self.parse_type_annotation();
            return_type_str = Some(typ.node.to_string());
            return_ty = Some(typ);
        }

        let mut stmt_list = Vec::new();

        self.bump_token(TokenKind::OpenDelim(DelimToken::Brace));

        // NEWLINE _INDENT
        let has_newline = if self.token.kind == TokenKind::Newline {
            self.skip_newlines();

            if self.token.kind == TokenKind::Indent {
                self.bump();
            } else {
                self.sess
                    .struct_token_error(&[&TokenKind::Indent.into()], self.token)
            }
            true
        } else {
            false
        };

        loop {
            if matches!(
                self.token.kind,
                TokenKind::CloseDelim(DelimToken::Brace) | TokenKind::Dedent
            ) {
                break;
            }
            if let Some(stmt) = self.parse_stmt() {
                stmt_list.push(stmt);
                self.skip_newlines();
            }
        }

        // _DEDENT
        if has_newline {
            if self.token.kind == TokenKind::Dedent {
                self.bump();
            } else {
                self.sess
                    .struct_token_error(&[&TokenKind::Dedent.into()], self.token)
            }
        }

        self.bump_token(TokenKind::CloseDelim(DelimToken::Brace));

        Box::new(Node::node(
            Expr::Lambda(LambdaExpr {
                args,
                return_type_str,
                return_ty,
                body: stmt_list,
            }),
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    /// Return type of the lambda
    fn parse_lambda_type(&mut self) -> String {
        self.bump();

        if self.token.is_keyword(kw::Type) {
            self.bump();

            // rules: append strings util a left brace. panic on a '\n'.
            let mut s = String::new();

            while let TokenKind::Literal(lt) = self.token.kind {
                let token_str = lt.symbol.as_str();
                if token_str == "\n" {
                    self.sess
                        .struct_span_error("cross line type is not supported.", self.token.span)
                }

                s.push_str(lt.symbol.as_str())
            }

            s.to_string()
        } else {
            self.sess
                .struct_token_error(&[&kw::Type.into()], self.token)
        }
    }

    /// Syntax:
    /// paren_expr: LEFT_PARENTHESES expr RIGHT_PARENTHESES
    fn parse_paren_expr(&mut self) -> NodeRef<Expr> {
        let token = self.token;
        self.bump();

        let expr = self.parse_expr();
        match self.token.kind {
            TokenKind::CloseDelim(DelimToken::Paren) => {
                self.bump();
            }
            _ => self.sess.struct_token_error(
                &[&token::TokenKind::CloseDelim(token::DelimToken::Paren).into()],
                self.token,
            ),
        }

        Box::new(Node::node(
            Expr::Paren(ParenExpr { expr }),
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    /// Syntax:
    /// arguments: argument (COMMA argument)*
    fn parse_arguments_expr(&mut self) -> (Vec<NodeRef<Expr>>, Vec<NodeRef<Keyword>>) {
        let mut args: Vec<NodeRef<Expr>> = Vec::new();
        let mut keywords: Vec<NodeRef<Keyword>> = Vec::new();
        let mut has_keyword = false;

        loop {
            match self.parse_argument_expr() {
                Either::Left(expr) => {
                    args.push(Box::new(expr));
                    if has_keyword {
                        self.sess.struct_span_error(
                            "positional argument follows keyword argument.",
                            self.token.span,
                        )
                    }
                }
                Either::Right(keyword) => {
                    keywords.push(Box::new(keyword));
                    has_keyword = true;
                }
            }

            if self.token.kind == TokenKind::Comma {
                self.bump();
            } else {
                break;
            }
        }

        (args, keywords)
    }

    /// Syntax:
    /// argument: expr | Identifier ASSIGN expr
    fn parse_argument_expr(&mut self) -> Either<Node<Expr>, Node<Keyword>> {
        let token = self.token;

        // Identifier
        let arg_lo = self.token;
        let expr = self.parse_expr();
        let arg_hi = self.prev_token;

        match self.token.kind {
            TokenKind::Assign => {
                self.bump();

                let arg = match &expr.node {
                    Expr::Identifier(x) => x.clone(),
                    _ => self
                        .sess
                        .struct_token_error(&[&TokenKind::ident_value()], self.token),
                };

                // expr
                let value = self.parse_expr();

                either::Right(Node::node(
                    Keyword {
                        arg: Box::new(Node::node(arg, self.sess.struct_token_loc(arg_lo, arg_hi))),
                        value: Some(value),
                    },
                    self.sess.struct_token_loc(token, self.prev_token),
                ))
            }
            _ => either::Left(*expr),
        }
    }

    /// ~~~ Schema

    /// Syntax:
    /// decorator_expr: identifier [call_suffix]
    fn parse_decorator_expr(&mut self) -> NodeRef<Expr> {
        let token = self.token;
        let func = self.parse_identifier_expr();

        // LEFT_PARENTHESES
        match self.token.kind {
            TokenKind::OpenDelim(DelimToken::Paren) => {
                self.bump();

                self.parse_call_expr(func)
            }
            _ => Box::new(Node::node(
                Expr::Call(CallExpr {
                    func,
                    args: Vec::new(),
                    keywords: Vec::new(),
                }),
                self.sess.struct_token_loc(token, self.prev_token),
            )),
        }
    }

    /// Syntax:
    /// check_expr: simple_expr [IF simple_expr] [COMMA primary_expr] NEWLINE
    pub(crate) fn parse_check_expr(&mut self) -> NodeRef<Expr> {
        let token = self.token;

        // expr
        let test = self.parse_simple_expr();
        // [IF expr]
        let if_cond = if self.token.is_keyword(kw::If) {
            self.bump();
            Some(self.parse_simple_expr())
        } else {
            None
        };

        // [COMMA primary_expr]
        let msg = if self.token.kind == TokenKind::Comma {
            self.bump();

            Some(self.parse_primary_expr())
        } else {
            None
        };

        Box::new(Node::node(
            Expr::Check(CheckExpr { test, if_cond, msg }),
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    /// ~~~ Id

    fn parse_identifier(&mut self) -> NodeRef<Identifier> {
        let token = self.token;
        let mut names = Vec::new();
        let ident = self.token.ident();
        match ident {
            Some(id) => {
                names.push(id.as_str().to_string());
                self.bump();
            }
            None => self
                .sess
                .struct_token_error(&[&TokenKind::ident_value()], self.token),
        }

        loop {
            let token = self.token;
            match token.kind {
                TokenKind::Dot => {
                    self.bump();
                    let ident = self.token.ident();
                    match ident {
                        Some(id) => {
                            names.push(id.as_str().to_string());
                            self.bump();
                        }
                        None => self
                            .sess
                            .struct_token_error(&[&TokenKind::ident_value()], self.token),
                    }
                }
                _ => break,
            }
        }

        Box::new(Node::node(
            Identifier {
                names,
                pkgpath: "".to_string(),
                ctx: ExprContext::Load,
            },
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    /// ~~~ Lit

    /// Syntax:
    /// number: DEC_NUMBER [multiplier] | HEX_NUMBER | BIN_NUMBER | OCT_NUMBER | FLOAT_NUMBER
    fn parse_num_expr(&mut self, lk: token::Lit) -> NodeRef<Expr> {
        let token = self.token;

        let (binary_suffix, value) = match lk.kind {
            token::LitKind::Integer => {
                let value = bytes_to_int(lk.symbol.as_str().as_bytes(), 0).unwrap();

                match lk.suffix {
                    Some(suffix) => (suffix.as_str().try_into().ok(), NumberLitValue::Int(value)),
                    None => (None, NumberLitValue::Int(value)),
                }
            }
            token::LitKind::Float => {
                let value = lk.symbol.as_str().parse().unwrap();
                (None, NumberLitValue::Float(value))
            }
            _ => self.sess.struct_token_error(
                &[
                    &token::LitKind::Integer.into(),
                    &token::LitKind::Float.into(),
                ],
                self.token,
            ),
        };

        self.bump();

        Box::new(Node::node(
            Expr::NumberLit(NumberLit {
                binary_suffix,
                value,
            }),
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }

    /// Syntax:
    /// string: STRING | LONG_STRING
    pub(crate) fn parse_str_expr(&mut self, lk: token::Lit) -> NodeRef<Expr> {
        let token = self.token;

        let (is_long_string, raw_value, value) = match lk.kind {
            token::LitKind::Str { is_long_string, .. } => {
                let value = lk.symbol.as_str().to_string();
                let raw_value = lk
                    .raw
                    .map_or("".to_string(), |raw| raw.as_str().to_string());
                (is_long_string, raw_value, value)
            }
            _ => self.sess.struct_token_error(
                &[&token::LitKind::Str {
                    is_long_string: false,
                    is_raw: false,
                }
                .into()],
                self.token,
            ),
        };

        self.bump();

        let loc = self.sess.struct_token_loc(token, self.prev_token);
        let lit = StringLit {
            is_long_string,
            value,
            raw_value,
        };

        if let Some(joined_str) = self.parse_joined_string(&lit, token.span.lo()) {
            Box::new(Node::node(
                Expr::JoinedString(joined_str),
                self.sess.struct_token_loc(token, self.prev_token),
            ))
        } else {
            Box::new(Node::node(Expr::StringLit(lit), loc))
        }
    }

    /// Syntax:
    /// constant : TRUE | FALSE | NONE | UNDEFINED
    fn parse_constant_expr(&mut self, lk: token::LitKind) -> NodeRef<Expr> {
        let token = self.token;

        let value = match lk {
            token::LitKind::Bool => {
                if self.token.is_keyword(kw::True) {
                    NameConstant::True
                } else if self.token.is_keyword(kw::False) {
                    NameConstant::False
                } else {
                    self.sess
                        .struct_token_error(&[&token::LitKind::Bool.into()], self.token)
                }
            }
            token::LitKind::None => NameConstant::None,
            token::LitKind::Undefined => NameConstant::Undefined,
            _ => self.sess.struct_token_error(
                &[
                    &token::LitKind::Bool.into(),
                    &token::LitKind::None.into(),
                    &token::LitKind::Undefined.into(),
                ],
                self.token,
            ),
        };

        self.bump();

        Box::new(Node::node(
            Expr::NameConstantLit(NameConstantLit { value }),
            self.sess.struct_token_loc(token, self.prev_token),
        ))
    }
}
