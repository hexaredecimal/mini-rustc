use super::Parser;
use crate::{
    ast::{self, Expr, ExprKind, Ident, UnOp},
    lexer::{self, Token, TokenKind},
};

pub fn is_expr_start(token: &Token) -> bool {
    matches!(
        token.kind,
        TokenKind::NumLit(_)
            | TokenKind::StrLit(_)
            | TokenKind::Ident(_)
            | TokenKind::OpenParen
            | TokenKind::OpenBrace
            | TokenKind::BinOp(lexer::BinOp::Plus | lexer::BinOp::Minus)
            | TokenKind::Return
            | TokenKind::True
            | TokenKind::False
            | TokenKind::If
            | TokenKind::Unsafe
    )
}

impl Parser {
    /// expr ::= assign
    pub fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_assign()
    }

    /// ifExpr ::= "if" expr  block ("else" (block | ifExpr))?
    fn parse_if_expr(&mut self) -> Option<Expr> {
        if !self.skip_expected_token(TokenKind::If) {
            eprintln!("Expected \"if\", but found {:?}", self.peek_token());
            return None;
        }
        let cond = self.parse_expr()?;
        let then_block = self.parse_block()?;
        let t = self.peek_token()?;
        let els = if t.kind == TokenKind::Else {
            self.skip_token();
            let t = self.peek_token()?;
            if t.kind == TokenKind::If {
                Some(self.parse_if_expr()?)
            } else {
                Some(Expr {
                    kind: ExprKind::Block(self.parse_block()?),
                    id: self.get_next_id(),
                })
            }
        } else {
            None
        };

        Some(Expr {
            kind: ExprKind::If(
                Box::new(cond),
                Box::new(Expr {
                    kind: ExprKind::Block(then_block),
                    id: self.get_next_id(),
                }),
                els.map(Box::new),
            ),
            id: self.get_next_id(),
        })
    }

    /// assign ::= equality ("=" assign)?
    fn parse_assign(&mut self) -> Option<Expr> {
        let lhs = self.parse_binary_equality()?;
        let t = self.lexer.peek_token()?;
        if t.kind != TokenKind::Eq {
            return Some(lhs);
        }
        self.skip_token();
        let rhs = self.parse_assign()?;
        Some(Expr {
            kind: ExprKind::Assign(Box::new(lhs), Box::new(rhs)),
            id: self.get_next_id(),
        })
    }

    /// equality ::= relational (("=="|"!=") equality)?
    fn parse_binary_equality(&mut self) -> Option<Expr> {
        let lhs = self.parse_binary_relational()?;
        let t = self.lexer.peek_token()?;
        let binop = match t.kind {
            TokenKind::BinOp(lexer::BinOp::Eq) => ast::BinOp::Eq,
            TokenKind::BinOp(lexer::BinOp::Ne) => ast::BinOp::Ne,
            _ => {
                return Some(lhs);
            }
        };
        self.lexer.skip_token();

        let rhs = self.parse_binary_equality()?;

        Some(Expr {
            kind: ExprKind::Binary(binop, Box::new(lhs), Box::new(rhs)),
            id: self.get_next_id(),
        })
    }

    /// relational ::= add (("=="|"!=") relational)?
    fn parse_binary_relational(&mut self) -> Option<Expr> {
        let lhs = self.parse_binary_add()?;
        let t = self.lexer.peek_token()?;
        let binop = match t.kind {
            TokenKind::BinOp(lexer::BinOp::Lt) => ast::BinOp::Lt,
            TokenKind::BinOp(lexer::BinOp::Gt) => ast::BinOp::Gt,
            _ => {
                return Some(lhs);
            }
        };
        self.lexer.skip_token();

        let rhs = self.parse_binary_relational()?;

        Some(Expr {
            kind: ExprKind::Binary(binop, Box::new(lhs), Box::new(rhs)),
            id: self.get_next_id(),
        })
    }

    /// add ::= mul ("+"|"-") add
    fn parse_binary_add(&mut self) -> Option<Expr> {
        let lhs = self.parse_binary_mul()?;
        let t = self.lexer.peek_token()?;
        let binop = match t.kind {
            TokenKind::BinOp(lexer::BinOp::Plus) => ast::BinOp::Add,
            TokenKind::BinOp(lexer::BinOp::Minus) => ast::BinOp::Sub,
            _ => {
                return Some(lhs);
            }
        };
        self.lexer.skip_token();

        let rhs = self.parse_binary_add()?;

        Some(Expr {
            kind: ExprKind::Binary(binop, Box::new(lhs), Box::new(rhs)),
            id: self.get_next_id(),
        })
    }

    /// mul ::= unary "*" mul
    fn parse_binary_mul(&mut self) -> Option<Expr> {
        let lhs = self.parse_binary_unary()?;
        let t = self.lexer.peek_token()?;
        let binop = match t.kind {
            TokenKind::BinOp(lexer::BinOp::Star) => ast::BinOp::Mul,
            _ => {
                return Some(lhs);
            }
        };
        self.lexer.skip_token();

        let rhs = self.parse_binary_mul()?;

        Some(Expr {
            kind: ExprKind::Binary(binop, Box::new(lhs), Box::new(rhs)),
            id: self.get_next_id(),
        })
    }

    /// unary ::= ("+"|"-") primary
    fn parse_binary_unary(&mut self) -> Option<Expr> {
        let t = self.lexer.peek_token()?;
        let unup = match &t.kind {
            TokenKind::BinOp(lexer::BinOp::Plus) => UnOp::Plus,
            TokenKind::BinOp(lexer::BinOp::Minus) => UnOp::Minus,
            _ => {
                return self.parse_binary_primary();
            }
        };
        // skip unary op token
        self.skip_token();

        let primary = self.parse_binary_primary()?;
        Some(Expr {
            kind: ExprKind::Unary(unup, Box::new(primary)),
            id: self.get_next_id(),
        })
    }

    /// primary ::= num | true | false | stringLit
    ///     | ident | callExpr | indexExpr | ifExpr
    ///     | returnExpr | "(" expr ")"
    ///     | unsafeBlock | block
    ///     | fieldExpr | structExpr
    /// returnExpr ::= "return" expr
    fn parse_binary_primary(&mut self) -> Option<Expr> {
        let t = &self.lexer.peek_token().unwrap();
        let mut expr = match t.kind {
            TokenKind::NumLit(n) => {
                self.skip_token();
                Expr {
                    kind: ExprKind::NumLit(n),
                    id: self.get_next_id(),
                }
            }
            TokenKind::True => {
                self.skip_token();
                Expr {
                    kind: ExprKind::BoolLit(true),
                    id: self.get_next_id(),
                }
            }
            TokenKind::False => {
                self.skip_token();
                Expr {
                    kind: ExprKind::BoolLit(false),
                    id: self.get_next_id(),
                }
            }
            TokenKind::StrLit(_) => {
                let TokenKind::StrLit(s) = self.skip_token().unwrap().kind else { unreachable!() };
                Expr {
                    kind: ExprKind::StrLit(s),
                    id: self.get_next_id(),
                }
            }
            TokenKind::If => self.parse_if_expr()?,
            TokenKind::Return => {
                self.skip_token();
                let e = self.parse_expr()?;
                Expr {
                    kind: ExprKind::Return(Box::new(e)),
                    id: self.get_next_id(),
                }
            }
            TokenKind::Ident(_) => self.parse_ident_or_struct_expr()?,
            TokenKind::OpenParen => {
                // skip '('
                self.skip_token().unwrap();
                let t = self.peek_token().unwrap();
                if t.kind == TokenKind::CloseParen {
                    // skip '('
                    self.skip_token().unwrap();
                    Expr {
                        kind: ExprKind::Unit,
                        id: self.get_next_id(),
                    }
                } else {
                    let expr = self.parse_expr()?;
                    // skip ')'
                    if !self.skip_expected_token(TokenKind::CloseParen) {
                        eprintln!("Expected ')', but found {:?}", self.peek_token());
                        return None;
                    }
                    expr
                }
            }
            // unsafe block expression
            // TODO: Should AST node have `unsafe` info?
            TokenKind::Unsafe => {
                self.skip_token().unwrap();
                Expr {
                    kind: ExprKind::Block(self.parse_block()?),
                    id: self.get_next_id(),
                }
            }
            // block expression
            TokenKind::OpenBrace => Expr {
                kind: ExprKind::Block(self.parse_block()?),
                id: self.get_next_id(),
            },
            _ => {
                eprintln!("Expected num or (expr), but found {:?}", t);
                return None;
            }
        };
        // deal with tailing `(...)` (func call), `[...]` (indexing), .ident (field access)
        // FIXME: disambiguity: () () => FuncCall or ExprStmt ExprStmt
        loop {
            let t = self.peek_token()?;
            match &t.kind {
                TokenKind::OpenParen => {
                    expr = self.parse_call_expr(expr)?;
                }
                TokenKind::OpenBracket => expr = self.parse_index_expr(expr)?,
                TokenKind::Dot => expr = self.parse_field_expr(expr)?,
                _ => break,
            }
        }
        Some(expr)
    }

    /// ident | structExpr
    fn parse_ident_or_struct_expr(&mut self) -> Option<Expr> {
        let ident = self.parse_ident().unwrap();
        let t = self.peek_token().unwrap();
        if let TokenKind::OpenBrace = t.kind {
            self.parse_struct_expr(ident)
        } else {
            Some(Expr {
                kind: ExprKind::Ident(ident),
                id: self.get_next_id(),
            })
        }
    }

    /// structExpr ::= ident "{" structExprFields? "}"
    /// NOTE: first ident is already parsed
    fn parse_struct_expr(&mut self, ident: Ident) -> Option<Expr> {
        if !self.skip_expected_token(TokenKind::OpenBrace) {
            eprintln!("Expected '{{', but found {:?}", self.peek_token());
            return None;
        }

        let fields = if matches!(self.peek_token().unwrap().kind, TokenKind::Ident(_)) {
            self.parse_struct_expr_fields()?
        } else {
            vec![]
        };

        if !self.skip_expected_token(TokenKind::CloseBrace) {
            eprintln!("Expected '}}', but found {:?}", self.peek_token());
            return None;
        }
        Some(Expr {
            kind: ExprKind::Struct(ident, fields),
            id: self.get_next_id(),
        })
    }

    /// structExprFields ::= structExprField ("," structExprField)* ","?
    fn parse_struct_expr_fields(&mut self) -> Option<Vec<(Ident, Box<Expr>)>> {
        let mut fds = vec![];
        fds.push(self.parse_struct_expr_field()?);

        while matches!(self.peek_token()?.kind, TokenKind::Comma) {
            self.skip_token();
            if matches!(self.peek_token().unwrap().kind, TokenKind::Ident(_)) {
                fds.push(self.parse_struct_expr_field()?);
            }
        }
        Some(fds)
    }

    /// structExprField ::= ident ":" expr
    fn parse_struct_expr_field(&mut self) -> Option<(Ident, Box<Expr>)> {
        let ident = self.parse_ident()?;
        if !self.skip_expected_token(TokenKind::Colon) {
            eprintln!("Expected ':', but found {:?}", self.peek_token());
            return None;
        }
        let expr = self.parse_expr()?;
        Some((ident, Box::new(expr)))
    }

    /// callExpr ::= primary "(" callParams? ")"
    /// NOTE: first primary is already parsed
    fn parse_call_expr(&mut self, fn_expr: Expr) -> Option<Expr> {
        // skip '('
        self.skip_token();
        let args = if self.peek_token()?.kind == TokenKind::CloseParen {
            vec![]
        } else {
            self.parse_call_params()?
        };

        if !self.skip_expected_token(TokenKind::CloseParen) {
            eprintln!("Expected ')', but found {:?}", self.peek_token());
            return None;
        }
        Some(Expr {
            kind: ExprKind::Call(Box::new(fn_expr), args),
            id: self.get_next_id(),
        })
    }

    /// callParams ::= callParam ("," callParam)* ","?
    /// callParam = expr
    fn parse_call_params(&mut self) -> Option<Vec<Expr>> {
        let mut args = vec![];
        args.push(self.parse_expr()?);

        while matches!(self.peek_token()?.kind, TokenKind::Comma) {
            self.skip_token();
            if is_expr_start(self.peek_token()?) {
                args.push(self.parse_expr()?);
            }
        }
        Some(args)
    }

    /// indexExpr ::= priamry "[" expr "]"
    /// NOTE: first primary is already parsed
    fn parse_index_expr(&mut self, array_expr: Expr) -> Option<Expr> {
        // skip '['
        if !self.skip_expected_token(TokenKind::OpenBracket) {
            eprintln!("Expected '[', but found {:?}", self.peek_token());
            return None;
        }
        let index = self.parse_expr()?;

        // skip ']'
        if !self.skip_expected_token(TokenKind::CloseBracket) {
            eprintln!("Expected ']', but found {:?}", self.peek_token());
            return None;
        }
        Some(Expr {
            kind: ExprKind::Index(Box::new(array_expr), Box::new(index)),
            id: self.get_next_id(),
        })
    }

    /// fieldExpr ::= primary "(" callParams? ")"
    /// NOTE: first primary is already parsed
    fn parse_field_expr(&mut self, recv: Expr) -> Option<Expr> {
        // skip '.'
        self.skip_token();
        let fd = self.parse_ident()?;

        Some(Expr {
            kind: ExprKind::Field(Box::new(recv), fd),
            id: self.get_next_id(),
        })
    }
}
