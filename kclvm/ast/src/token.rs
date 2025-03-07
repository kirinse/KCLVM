//! KCL AST Affinity tokens.
//!
//! Tokens are designed based on the KCL AST.
//! Including indent and dedent tokens.
//! Not Include some tokens of low level tokens, such as ';', '..', '..=', '<-'.
pub use BinCmpToken::*;
pub use BinCmpToken::*;
pub use BinOpToken::*;
pub use DelimToken::*;
pub use LitKind::*;
pub use TokenKind::*;
pub use UnaryOpToken::*;

use kclvm_span::symbol::{Ident, Symbol};
use kclvm_span::{Span, DUMMY_SP};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CommentKind {
    /// "#"
    Line(Symbol),
}

#[derive(Clone, PartialEq, Hash, Debug, Copy)]
pub enum UnaryOpToken {
    /// "~"
    UTilde,

    /// "not"
    UNot,
}

#[derive(Clone, PartialEq, Hash, Debug, Copy)]
pub enum BinOpToken {
    /// "+"
    Plus,

    /// "-"
    Minus,

    /// "*"
    Star,

    /// "/"
    Slash,

    /// "%"
    Percent,

    /// "**"
    StarStar,

    /// "//"
    SlashSlash,

    /// "^"
    Caret,

    /// "&"
    And,

    /// "|"
    Or,

    /// "<<"
    Shl,

    /// ">>"
    Shr,
}

#[derive(Clone, PartialEq, Hash, Debug, Copy)]
pub enum BinCmpToken {
    /// "=="
    Eq,

    /// "!="
    NotEq,

    /// "<"
    Lt,

    /// "<="
    LtEq,

    /// ">"
    Gt,

    /// ">="
    GtEq,
}

/// A delimiter token.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Copy)]
pub enum DelimToken {
    /// A round parenthesis (i.e., `(` or `)`).
    Paren,
    /// A square bracket (i.e., `[` or `]`).
    Bracket,
    /// A curly brace (i.e., `{` or `}`).
    Brace,
    /// An empty delimiter.
    NoDelim,
}

/// A literal token.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Lit {
    pub kind: LitKind,
    pub symbol: Symbol,
    pub suffix: Option<Symbol>,
    pub raw: Option<Symbol>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LitKind {
    Bool,
    Integer,
    Float,
    Str { is_long_string: bool, is_raw: bool },
    None,
    Undefined,
    Err,
}

impl Into<String> for LitKind {
    fn into(self) -> String {
        let s = match self {
            Bool => "bool",
            Integer => "int",
            Float => "float",
            Str { .. } => "str",
            None => "None",
            Undefined => "Undefined",
            Err => "error",
        };

        s.to_string()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TokenKind {
    /* Expression-operator symbols. */
    UnaryOp(UnaryOpToken),
    BinOp(BinOpToken),
    BinOpEq(BinOpToken),
    BinCmp(BinCmpToken),

    /* Structural symbols */
    /// '@'
    At,
    /// '.'
    Dot,
    /// '...'
    DotDotDot,
    /// ','
    Comma,
    /// ':'
    Colon,
    /// '->'
    RArrow,
    /// '$'
    Dollar,
    /// '?'
    Question,
    /// '='
    Assign,
    /// An opening delimiter (e.g., `{`).
    OpenDelim(DelimToken),
    /// A closing delimiter (e.g., `}`).
    CloseDelim(DelimToken),

    /* Literals */
    Literal(Lit),

    /// Identifier token.
    Ident(Symbol),

    /// A comment token.
    DocComment(CommentKind),

    /// '\t' or ' '
    Indent,

    /// Remove an indent
    Dedent,

    /// '\n'
    Newline,

    Dummy,

    Eof,
}

impl TokenKind {
    pub fn ident_value() -> String {
        "identifier".to_string()
    }

    pub fn literal_value() -> String {
        "literal".to_string()
    }
}

impl Into<String> for TokenKind {
    fn into(self) -> String {
        let s = match self {
            UnaryOp(unary_op) => match unary_op {
                UTilde => "~",
                UNot => "not",
            },
            BinOp(bin_op) => match bin_op {
                Plus => "+",
                Minus => "-",
                Star => "*",
                Slash => "/",
                Percent => "%",
                StarStar => "**",
                SlashSlash => "//",
                Caret => "^",
                And => "&",
                Or => "|",
                Shl => "<<",
                Shr => ">>",
            },
            BinOpEq(bin_op_eq) => match bin_op_eq {
                Plus => "+=",
                Minus => "-=",
                Star => "*=",
                Slash => "/=",
                Percent => "%=",
                StarStar => "**=",
                SlashSlash => "//=",
                Caret => "^=",
                And => "&=",
                Or => "|=",
                Shl => "<<=",
                Shr => ">>=",
            },
            BinCmp(bin_cmp) => match bin_cmp {
                Eq => "==",
                NotEq => "!=",
                Lt => "<",
                LtEq => "<=",
                Gt => ">",
                GtEq => ">=",
            },
            At => "@",
            Dot => ".",
            DotDotDot => "...",
            Comma => ",",
            Colon => ":",
            RArrow => "->",
            Dollar => "$",
            Question => "?",
            Assign => "=",
            OpenDelim(delim) => match delim {
                Paren => "(",
                Bracket => "[",
                Brace => "{",
                NoDelim => "open_no_delim",
            },
            CloseDelim(delim) => match delim {
                Paren => ")",
                Bracket => "]",
                Brace => "}",
                NoDelim => "close_no_delim",
            },
            Literal(lit) => match lit.kind {
                Bool => "bool",
                Integer => "integer",
                Float => "float",
                Str { .. } => "string",
                None => "None",
                Undefined => "Undefined",
                Err => "err",
            },
            TokenKind::Ident(_) => "identifier",
            DocComment(kind) => match kind {
                CommentKind::Line(_) => "inline_comment",
            },
            Indent => "indent",
            Dedent => "dedent",
            Newline => "newline",
            Dummy => "dummy",
            Eof => "eof",
        };
        s.to_string()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Into<String> for Token {
    fn into(self) -> String {
        match self.kind {
            Literal(lk) => {
                let sym = lk.symbol.as_str().to_string();

                match lk.suffix {
                    Some(suf) => sym + &suf.as_str(),
                    _other_none => sym,
                }
            }
            _ => self.kind.into(),
        }
    }
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }

    /// Some token that will be thrown away later.
    pub fn dummy() -> Self {
        Token::new(TokenKind::Dummy, DUMMY_SP)
    }

    /// Returns an identifier if this token is an identifier.
    pub fn ident(&self) -> Option<Ident> {
        match self.kind {
            Ident(name) => Some(Ident::new(name, self.span)),
            _ => std::option::Option::None,
        }
    }

    pub fn is_keyword(&self, kw: Symbol) -> bool {
        self.run_on_ident(|id| id.name == kw)
    }

    fn run_on_ident(&self, pred: impl FnOnce(Ident) -> bool) -> bool {
        match self.ident() {
            Some(id) => pred(id),
            _ => false,
        }
    }
}

impl PartialEq<TokenKind> for Token {
    fn eq(&self, rhs: &TokenKind) -> bool {
        self.kind == *rhs
    }
}
