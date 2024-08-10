use std::{
    fmt,
    fmt::Display,
};

use crate::common::Fraction;

/// Bool -> true | false
#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct BoolLiteral(pub bool);

/// Number -> [0-9]+('/'[0-9]+)?
#[derive(Debug, PartialEq, Eq)]
pub enum NumberLiteral {
    Unsigned(Fraction<u64>),
    Signed(Fraction<i64>),
}

/// String -> "'" (QuotedChar)* "'" | UnquotedString
///
/// QuotedChar -> [^'] | "\\'"
///
/// UnquotedString -> [a-zA-Z0-9_\-:]+
#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct StringLiteral(pub String);

/// List -> "[" Values "]"
#[derive(Debug, PartialEq, Eq)]
pub struct List(pub Values);

/// Identifier -> IdentifierStart IdentifierChar*
///
/// IdentifierStart -> [a-zA-Z_\-]
///
/// IdentifierChar -> [a-zA-Z0-9_\-]
#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Identifier(pub String);

/// ValueExpr -> "expr(" Expr ")"
#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ValueExpr(pub Box<Expr>);

/// ValueFunctionCall -> "func_call(" FunctionCall ")"
#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ValueFunctionCall(pub FunctionCall);

/// FormattedString -> "str(" StringLiteral ("," Values)? ")"
#[derive(Debug, PartialEq, Eq)]
pub struct FormattedString {
    pub template: StringLiteral,
    pub args: Values,
}

// str("magnitude = {}", $num)

/// Var -> "$" Identifier ("." Identifier)*
#[derive(Debug, PartialEq, Eq)]
pub struct Var {
    pub name: Identifier,
    pub member_access: Vec<Identifier>,
}

impl Var {
    pub fn full_name(&self) -> String {
        let mut name = self.name.0.clone();
        for member in &self.member_access {
            name.push('.');
            name.push_str(&member.0);
        }
        name
    }
}

/// Value -> "undefined" | Bool | Number | String | List | Var | ValueExpr | ValueFunctionCall
#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    UndefinedLiteral,
    BoolLiteral(BoolLiteral),
    NumberLiteral(NumberLiteral),
    StringLiteral(StringLiteral),
    List(List),
    Var(Var),
    ValueExpr(ValueExpr),
    ValueFunctionCall(ValueFunctionCall),
    FormattedString(FormattedString),
}

/// Values -> "" | Value (" " Value)*
///
/// Delimiter may be comma or space depending on the context.
#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Values(pub Vec<Value>);

/// FunctionCall -> Identifier (":" Values)?
#[derive(Debug, PartialEq, Eq)]
pub struct FunctionCall {
    pub function: Identifier,
    pub args: Values,
}

/// Assignment -> Var "=" Expr
#[derive(Debug, PartialEq, Eq)]
pub struct Assignment {
    pub lhs: Var,
    pub rhs: Expr,
}

/// Operator.
///
/// Precedence is determined by the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Not,

    Exponent,

    Multiply,
    Divide,
    Modulo,

    Add,
    Subtract,

    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Has,
    HasAny,

    Equal,
    NotEqual,

    And,

    Or,
}

impl From<Operator> for &str {
    fn from(value: Operator) -> Self {
        match value {
            Operator::Not => "!",
            Operator::Exponent => "^",
            Operator::Multiply => "*",
            Operator::Divide => "/",
            Operator::Modulo => "%",
            Operator::Add => "+",
            Operator::Subtract => "-",
            Operator::LessThan => "<",
            Operator::LessThanOrEqual => "<=",
            Operator::GreaterThan => ">",
            Operator::GreaterThanOrEqual => ">=",
            Operator::Has => "has",
            Operator::HasAny => "hasany",
            Operator::Equal => "==",
            Operator::NotEqual => "!=",
            Operator::And => "and",
            Operator::Or => "or",
        }
    }
}

impl Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Into::<&str>::into(*self))
    }
}

/// Expr -> Value | PrefixUnaryExpr | BinaryExpr
#[derive(Debug, PartialEq, Eq)]
pub enum Expr {
    Value(Value),
    PrefixUnaryExpr(PrefixUnaryExpr),
    BinaryExpr(BinaryExpr),
}

/// PrefixUnaryExpr -> Operator Expr
#[derive(Debug, PartialEq, Eq)]
pub struct PrefixUnaryExpr {
    pub ops: Vec<Operator>,
    pub expr: Box<Expr>,
}

/// BinaryExprRhs -> Operator Expr
#[derive(Debug, PartialEq, Eq)]
pub struct BinaryExprRhs {
    pub op: Operator,
    pub expr: Box<Expr>,
}

/// BinaryExpr -> Expr (BinaryExprRhs)+
#[derive(Debug, PartialEq, Eq)]
pub struct BinaryExpr {
    pub lhs: Box<Expr>,
    pub rhs: Vec<BinaryExprRhs>,
}

/// IfStatement -> "if" Expr ":"
#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct IfStatement(pub Expr);

/// IfStatement -> "else" (IfStatement | ":")
#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ElseIfStatement(pub Option<IfStatement>);

/// ForEachStatement -> "foreach" Var "in" Value ":"
#[derive(Debug, PartialEq, Eq)]
pub struct ForEachStatement {
    pub var: Var,
    pub range: Value,
}

/// ReturnStatement -> "return" Value
#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ReturnStatement(pub Option<Value>);

/// ContinueStatement -> "continue"
#[derive(Debug, PartialEq, Eq)]
pub struct ContinueStatement;

/// Statement -> Empty | FunctionCall | Assignment | IfStatement | ElseIfStatement |
/// ForEachStatement | ReturnStatement
#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    Empty,
    FunctionCall(FunctionCall),
    Assignment(Assignment),
    IfStatement(IfStatement),
    ElseIfStatement(ElseIfStatement),
    ForEachStatement(ForEachStatement),
    ReturnStatement(ReturnStatement),
    Continue(ContinueStatement),
}
