pub mod visitor;

use crate::span::{Ident, Span};

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct NodeId {
    private: u32,
}

impl NodeId {
    pub fn new(id: u32) -> NodeId {
        NodeId { private: id }
    }
}

impl std::fmt::Debug for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.private)
    }
}

#[derive(Debug)]
pub struct Crate {
    pub items: Vec<Item>,
    pub id: NodeId,
}

#[derive(Debug)]
pub struct Item {
    pub kind: ItemKind,
}

#[derive(Debug)]
pub enum ItemKind {
    Func(Func),
    Struct(StructItem),
    ExternBlock(ExternBlock),
    Mod(Module),
    Impl(Impl), 
    TypeAlias(Type), 
}

#[derive(Debug)]
pub struct Type {
    pub name: Ident, 
    pub aliasof: Ty
}


#[derive(Debug)]
pub struct Impl {
    pub name: Ident, 
    pub methods: Vec<Func>, 
}

#[derive(Debug)]
pub struct Module {
    pub name: Ident,
    pub items: Vec<Item>,
    pub id: NodeId,
}

#[derive(Debug)]
pub struct ExternBlock {
    pub funcs: Vec<Func>,
}

#[derive(Debug)]
pub struct StructItem {
    pub ident: Ident,
    pub fields: Vec<(Ident, Ty)>,
    pub id: NodeId,
}

#[derive(Debug)]
pub struct Func {
    pub name: Ident,
    pub params: Vec<(Ident, Ty)>,
    pub ret_ty: Ty,
    /// Extern abi
    pub ext: Option<String>,
    pub body: Option<Block>,
    pub id: NodeId,
    pub variadic: bool, 
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
    pub mutable: bool, 
    pub ty: Option<Ty>,
    pub init: Option<Expr>,
}

#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub id: NodeId,
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
    Path(Path),
    Assign(Box<Expr>, Box<Expr>),
    Return(Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Block(Block),
    /// cond, then (only block expr), else
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    Index(Box<Expr>, Box<Expr>),
    Field(Box<Expr>, Ident),
    Struct(Path, Vec<(Ident, Box<Expr>)>),
    Array(Vec<Expr>),
    Cast(Box<Expr>, Ty),
    Ref(Box<Expr>), 
    Deref(Path), 
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Path {
    pub segments: Vec<Ident>,
    pub span: Span,
}

impl std::fmt::Debug for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`")?;
        for (i, seg) in self.segments.iter().enumerate() {
            if i != 0 {
                write!(f, "::")?;
            }
            write!(f, "{}", seg.symbol)?;
        }
        write!(f, "`")?;
        write!(f, " ({:?})", self.span)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
    pub id: NodeId,
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
    Array(Box<Ty>, usize),
    Adt(Path),
    Ref(Option<Region>, Box<Ty>),
    ConstPtr(Box<Ty>),
    Never,
}

pub type Region = String;
