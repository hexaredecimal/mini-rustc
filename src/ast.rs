#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
}

#[derive(Debug)]
pub enum ExprKind {
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    NumLit(u32),
}

#[derive(Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
}

#[derive(Debug)]
pub enum UnOp {
    Plus,
    Minus,
}
