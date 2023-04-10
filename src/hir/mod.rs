use crate::span::Span;
use std::rc::Rc;

pub mod visitor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirId {
    private: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId {
    private: u32,
}

#[derive(Debug)]
pub struct Crate {
    pub items: Vec<Item>,
    pub id: HirId,
}

#[derive(Debug)]
pub struct Item {
    pub kind: ItemKind,
}

#[derive(Debug)]
pub enum ItemKind {
    Func(Func),
    Struct(StructItem),
    Mod(Module),
}

#[derive(Debug)]
pub struct Module {
    pub name: Ident,
    pub items: Vec<Item>,
    pub id: HirId,
}

#[derive(Debug)]
pub struct ForeignMod {
    pub funcs: Vec<Func>,
}

#[derive(Debug)]
pub struct StructItem {
    pub ident: Ident,
    pub fields: Vec<(Ident, Rc<Ty>)>,
    pub id: HirId,
}

#[derive(Debug)]
pub struct Func {
    pub name: Ident,
    pub params: Vec<(Ident, Rc<Ty>)>,
    pub ret_ty: Rc<Ty>,
    /// Extern abi
    pub ext: Option<String>,
    pub body: Option<Block>,
    pub id: NodeId,
}

#[derive(Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub id: NodeId,
    pub span: Span,
}

#[derive(Debug)]
pub enum StmtKind {
    /// Expression without trailing semicolon
    Expr(Box<Expr>),
    /// Expression with trailing semicolon
    Semi(Box<Expr>),
    Let(LetStmt),
}

#[derive(Debug)]
pub struct LetStmt {
    pub ident: Ident,
    pub ty: Option<Ty>,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct Ident {
    // TODO: remove symbol and span
    // add ident: crate::span::Ident
    pub symbol: Rc<String>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub id: HirId,
    pub span: Span,
}

#[derive(Debug)]
pub enum ExprKind {
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    NumLit(u32),
    BoolLit(bool),
    StrLit(String),
    Unit,
    Ident(Ident),
    Assign(Box<Expr>, Box<Expr>),
    Return(Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Block(Block),
    /// cond, then (only block expr), else
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    Index(Box<Expr>, Box<Expr>),
    Field(Box<Expr>, Ident),
    Struct(Ident, Vec<(Ident, Box<Expr>)>),
    Array(Vec<Expr>),
}

#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
    pub id: HirId,
}

#[derive(Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Eq,
    Ne,
    Gt,
    Lt,
}

#[derive(Debug)]
pub enum UnOp {
    Plus,
    Minus,
}

#[derive(Debug)]
pub struct Ty {
    pub kind: TyKind,
    pub span: Span,
}

#[derive(Debug)]
pub enum TyKind {
    Unit,
    Bool,
    I32,
    Str,
    Array(Rc<Ty>, usize),
    Adt(Rc<String>),
    Ref(Option<Region>, Rc<Ty>),
    Never,
}

pub type Region = String;