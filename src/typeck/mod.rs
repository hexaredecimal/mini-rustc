use crate::analysis::Ctxt;
use crate::ast::{self, BinOp, Crate, ExprKind, Ident, LetStmt, StmtKind};
use crate::ty::Ty;
use std::collections::HashMap;
use std::rc::Rc;

pub fn typeck(ctx: &mut Ctxt, krate: &Crate) -> Result<(), Vec<String>> {
    let mut checker = TypeChecker::new(ctx);
    ast::visitor::go(&mut checker, krate);
    if checker.errors.is_empty() {
        Ok(())
    } else {
        Err(checker.errors)
    }
}

struct TypeChecker<'chk> {
    // To allow shadowing, this contains func params
    ident_ty_mappings: HashMap<&'chk String, Rc<Ty>>,
    current_return_type: Option<&'chk Ty>,
    ctx: &'chk mut Ctxt,
    errors: Vec<String>,
}

impl<'ctx, 'chk: 'ctx> TypeChecker<'ctx> {
    fn new(ctx: &'ctx mut Ctxt) -> Self {
        TypeChecker {
            ident_ty_mappings: HashMap::new(),
            current_return_type: None,
            ctx,
            errors: vec![],
        }
    }

    fn error(&mut self, e: String) {
        self.errors.push(e);
    }

    fn insert_ident_type(&mut self, symbol: &'chk String, ty: Rc<Ty>) {
        self.ident_ty_mappings.insert(symbol, Rc::clone(&ty));
    }

    fn get_ident_type(&mut self, ident: &Ident) -> Option<Rc<Ty>> {
        self.ident_ty_mappings.get(&ident.symbol).map(Rc::clone)
    }

    fn peek_return_type(&self) -> &Ty {
        self.current_return_type.as_ref().unwrap()
    }

    fn push_return_type(&mut self, ty: &'chk Ty) {
        self.current_return_type = Some(ty);
    }

    fn pop_return_type(&mut self) {
        self.current_return_type = None;
    }
}

impl<'ctx> ast::visitor::Visitor<'ctx> for TypeChecker<'ctx> {
    fn visit_func(&mut self, func: &'ctx ast::Func) {
        self.push_return_type(&func.ret_ty);
        // TODO: param type
        let func_ty = Rc::new(Ty::Fn(vec![], Rc::clone(&func.ret_ty)));
        /*
        self.ctx.set_fn_type(
            func.name.symbol.clone(),
            Rc::clone(&func_ty),
        );
        */
        self.insert_ident_type(&func.name.symbol, func_ty);
        for (param, param_ty) in &func.params {
            self.insert_ident_type(&param.symbol, Rc::clone(param_ty));
        }
    }
    fn visit_func_post(&mut self, _func: &'ctx ast::Func) {
        self.pop_return_type();
    }

    fn visit_stmt_post(&mut self, stmt: &'ctx ast::Stmt) {
        // TODO: typecheck StmtKind::Semi and StmtKind::Expr
        let ty: Rc<Ty> = match &stmt.kind {
            StmtKind::Let(LetStmt { ident, ty }) => {
                self.insert_ident_type(&ident.symbol, Rc::clone(ty));
                Rc::clone(ty)
            }
            StmtKind::Semi(_) => Rc::new(Ty::Unit),
            StmtKind::Expr(expr) => self.ctx.get_type(expr.id),
        };
        self.ctx.insert_type(stmt.id, ty);
    }

    // use post order
    fn visit_expr_post(&mut self, expr: &'ctx ast::Expr) {
        let ty: Rc<Ty> = match &expr.kind {
            ExprKind::Assign(l, r) => {
                let lhs_ty = &self.ctx.get_type(l.id);
                let rhs_ty = &self.ctx.get_type(r.id);
                if **lhs_ty == **rhs_ty {
                    Rc::new(Ty::Unit)
                } else {
                    self.error(format!("Cannot assign {:?} to {:?}", rhs_ty, lhs_ty));
                    Rc::new(Ty::Error)
                }
            }
            ExprKind::Binary(op, l, r) => {
                let lhs_ty = &self.ctx.get_type(l.id);
                let rhs_ty = &self.ctx.get_type(r.id);
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul => {
                        if **lhs_ty == Ty::I32 && **rhs_ty == Ty::I32 {
                            Rc::new(Ty::I32)
                        } else {
                            self.error("Both lhs and rhs must be type of i32".to_string());
                            Rc::new(Ty::Error)
                        }
                    }
                    BinOp::Gt | BinOp::Lt => {
                        if **lhs_ty == Ty::I32 && **rhs_ty == Ty::I32 {
                            Rc::new(Ty::Bool)
                        } else {
                            self.error("Both lhs and rhs must be type of i32".to_string());
                            Rc::new(Ty::Error)
                        }
                    }
                    BinOp::Eq | BinOp::Ne => {
                        if **lhs_ty == Ty::I32 && **rhs_ty == Ty::I32 {
                            Rc::new(Ty::Bool)
                        } else {
                            self.error("Both lhs and rhs must have the same type".to_string());
                            Rc::new(Ty::Error)
                        }
                    }
                }
            }
            ExprKind::NumLit(_) => Rc::new(Ty::I32),
            ExprKind::BoolLit(_) => Rc::new(Ty::Bool),
            ExprKind::Unary(_op, inner) => {
                let inner_ty = &self.ctx.get_type(inner.id);
                if **inner_ty == Ty::I32 {
                    Rc::new(Ty::I32)
                } else {
                    self.error("inner expr of unary must be type of i32".to_string());
                    Rc::new(Ty::Error)
                }
            }
            ExprKind::Ident(ident) => match self.get_ident_type(ident) {
                Some(ty) => ty,
                None => {
                    self.error(format!("Could not find type of {}", ident.symbol));
                    Rc::new(Ty::Error)
                }
            },
            ExprKind::Return(expr) => {
                let actual_ret_ty = self.ctx.get_type(expr.id);
                let expected_ret_ty = self.peek_return_type();
                if *actual_ret_ty == *expected_ret_ty {
                    Rc::new(Ty::Never)
                } else {
                    self.error(format!(
                        "Expected {:?} type, but {:?} returned",
                        expected_ret_ty, actual_ret_ty
                    ));
                    Rc::new(Ty::Error)
                }
            }
            ExprKind::Call(expr, _args) => {
                // TODO: typecheck args
                let maybe_func_ty = self.ctx.get_type(expr.id);
                if let Ty::Fn(_param_ty, ret_ty) = &*maybe_func_ty {
                    Rc::clone(&ret_ty)
                } else {
                    self.error(format!("Expected fn type, but found {:?}", maybe_func_ty));
                    Rc::new(Ty::Error)
                }
            }
            ExprKind::Block(block) => {
                if let Some(stmt) = block.stmts.last() {
                    let last_stmt_ty = &self.ctx.get_type(stmt.id);
                    Rc::clone(last_stmt_ty)
                } else {
                    // no statement. Unit type
                    Rc::new(Ty::Unit)
                }
            }
            ExprKind::If(_cond, then, _els) => {
                // TODO: typecheck cond and els
                let then_ty = &self.ctx.get_type(then.id);
                Rc::clone(then_ty)
            }
            ExprKind::Index(ident, index) => {
                todo!()
            }
        };
        self.ctx.insert_type(expr.id, ty);
    }
}
