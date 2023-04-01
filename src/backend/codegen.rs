use std::collections::HashMap;

use super::frame_info::FrameInfo;
use crate::analysis::Ctxt;
use crate::ast::{
    BinOp, Crate, Expr, ExprKind, Func, Ident, ItemKind, LetStmt, Stmt, StmtKind, UnOp,
};
use crate::ty::{AdtDef, Ty};

const PARAM_REGISTERS: [&str; 6] = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];

pub fn codegen(ctx: &Ctxt, krate: &Crate) -> Result<(), ()> {
    let mut codegen = Codegen::new(ctx);
    codegen.go(krate)?;
    Ok(())
}

struct Codegen<'a> {
    ctx: &'a Ctxt,
    current_frame: Option<FrameInfo<'a>>,
    // String literal to label mappings
    // "some_lit" => .LCN
    str_label_mappings: HashMap<&'a String, String>,
    next_label_id: u32,
}

impl<'a> Codegen<'a> {
    fn new(ctx: &'a Ctxt) -> Self {
        Codegen {
            ctx,
            current_frame: None,
            str_label_mappings: HashMap::new(),
            next_label_id: 0,
        }
    }

    fn get_new_label_id(&mut self) -> u32 {
        let id = self.next_label_id;
        self.next_label_id += 1;
        id
    }

    fn push_current_frame(&mut self, frame: FrameInfo<'a>) {
        self.current_frame = Some(frame);
    }

    fn get_current_frame(&self) -> &FrameInfo {
        let Some(f) = &self.current_frame else {
            panic!("ICE");
        };
        f
    }

    fn pop_current_frame(&mut self) {
        if self.current_frame.is_none() {
            panic!("ICE: cannot pop the current frame");
        }
        self.current_frame = None;
    }

    fn go(&mut self, krate: &'a Crate) -> Result<(), ()> {
        println!(".intel_syntax noprefix");
        println!(".globl main");
        self.codegen_crate(krate)?;
        for (str, label) in self.str_label_mappings.iter() {
            println!("{label}:");
            println!("\t.ascii \"{str}\"");
            println!("\t.zero 1");
        }
        Ok(())
    }

    fn codegen_crate(&mut self, krate: &'a Crate) -> Result<(), ()> {
        for item in &krate.items {
            match &item.kind {
                ItemKind::Func(func) => {
                    self.codegen_func(func)?;
                }
                ItemKind::Struct(_) => (),
                ItemKind::ExternBlock(_) => (),
            }
        }
        Ok(())
    }

    fn codegen_func(&mut self, func: &'a Func) -> Result<(), ()> {
        // do not generate code for the func if it does not have its body
        if func.body.is_none() {
            return Ok(());
        }

        let frame = FrameInfo::compute(self.ctx, func);
        if self.ctx.dump_enabled {
            dbg!(&frame);
        }
        self.push_current_frame(frame);

        println!("{}:", func.name.symbol);
        self.codegen_func_prologue()?;
        if let Some(body) = &func.body {
            for stmt in &body.stmts {
                self.codegen_stmt(stmt)?;
            }
        }
        // codegen of the last stmt results the last computation result stored in rax
        self.codegen_func_epilogue(func);

        self.pop_current_frame();
        Ok(())
    }

    fn codegen_func_prologue(&self) -> Result<(), ()> {
        let frame = self.get_current_frame();
        println!("\tpush rbp");
        println!("\tmov rbp, rsp");
        println!("\tsub rsp, {}", frame.size);
        for (i, (_, local)) in frame.args.iter().enumerate() {
            println!("\tmov [rbp-{}], {}", local.offset, PARAM_REGISTERS[i]);
        }
        Ok(())
    }

    fn codegen_func_epilogue(&self, func: &'a Func) {
        // FIXME: remove this?
        if let Some(body) = &func.body {
            let block_ty = self.ctx.get_block_type(body);
            if *block_ty == Ty::Unit {
                println!("\tmov rax, 0");
            }
        }
        println!("\tmov rsp, rbp");
        println!("\tpop rbp");
        println!("\tret");
    }

    fn codegen_stmt(&mut self, stmt: &'a Stmt) -> Result<(), ()> {
        match &stmt.kind {
            StmtKind::Semi(expr) => {
                self.codegen_expr(expr)?;

                // In case of struct type, pop stack to clean it.
                // FIXME: necessary?
                let ty = self.ctx.get_type(expr.id);
                if ty.is_adt() {
                    let adt = self.ctx.lookup_adt_def(ty.get_adt_name().unwrap()).unwrap();
                    self.clean_adt_on_stack(adt);
                }
                Ok(())
            }
            StmtKind::Expr(expr) => {
                self.codegen_expr(expr)?;
                Ok(())
            }
            StmtKind::Let(LetStmt { ident, ty, init }) => {
                if let Some(init) = init {
                    self.codegen_assign_local_var(ident, ty, init)?;
                }
                Ok(())
            }
        }
    }

    /// Generate code for expression.
    /// Result is stored to al, eax, or rax. In case of al and eax, rax is zero-extended with al, or eax.
    /// If expr is ZST, rax is not set.
    /// If expr is ADT, all of its fields are pushed to the stack.
    fn codegen_expr(&mut self, expr: &'a Expr) -> Result<(), ()> {
        match &expr.kind {
            ExprKind::NumLit(n) => {
                println!("#lit");
                println!("\tmov rax, {}", n);
            }
            ExprKind::BoolLit(b) => {
                if *b {
                    println!("\tmov rax, 1");
                } else {
                    println!("\tmov rax, 0");
                }
            }
            ExprKind::StrLit(s) => {
                let label = format!(".LC{}", self.get_new_label_id());
                println!("\tmov rax, OFFSET FLAT:{label} # static str");
                // register the constant label
                if self.str_label_mappings.get(s).is_none() {
                    self.str_label_mappings.insert(s, label);
                }
            }
            ExprKind::Unit => {
                println!("\tmov rax, 0");
            }
            ExprKind::Unary(unop, inner_expr) => {
                println!("#unary");
                match unop {
                    UnOp::Plus => self.codegen_expr(inner_expr)?,
                    UnOp::Minus => {
                        // compile `-expr` as `0 - expr`
                        self.codegen_expr(inner_expr)?;
                        println!("\tmov rdi, rax");
                        println!("\tmov rax, 0");
                        println!("\tsub rax, rdi");
                    }
                }
            }
            ExprKind::Binary(binop, lhs, rhs) => {
                // use rax and rdi if rhs/lhs is size of 64bit
                let ax = "eax";
                let di = "edi";
                println!("#binary");
                self.codegen_expr(lhs)?;
                self.push();
                self.codegen_expr(rhs)?;
                self.push();
                self.pop("rdi");
                self.pop("rax");

                match binop {
                    BinOp::Add => {
                        println!("\tadd {}, {}", ax, di);
                    }
                    BinOp::Sub => {
                        println!("\tsub {}, {}", ax, di);
                    }
                    BinOp::Mul => {
                        // NOTE: Result is stored in rax
                        println!("\tmul {}", di);
                    }
                    BinOp::Eq => {
                        println!("\tcmp {}, {}", ax, di);
                        println!("\tsete al");
                        // zero extended to rax later
                    }
                    _ => todo!(),
                };
            }
            ExprKind::Ident(_) | ExprKind::Index(_, _) | ExprKind::Field(_, _) => {
                println!("#ident or index");
                self.codegen_addr(expr)?;
                println!("\tmov rax, [rax]");
            }
            ExprKind::Assign(lhs, rhs) => {
                self.codegen_assign(lhs, rhs)?;
            }
            ExprKind::Return(inner) => {
                self.codegen_expr(inner)?;
                println!("\tmov rsp, rbp");
                // TODO: remove this?
                let inner_ty = self.ctx.get_type(inner.id);
                if *inner_ty == Ty::Unit {
                    println!("\tmov rax, 0");
                }
                println!("\tpop rbp");
                println!("\tret");
            }
            ExprKind::Call(func, args) => {
                if args.len() > 6 {
                    todo!("number of args must be < 6");
                }
                for param in args {
                    // TODO: pass struct param via stack
                    // p16. https://www.uclibc.org/docs/psABI-x86_64.pdf
                    self.codegen_expr(param)?;
                    self.push();
                }
                for i in 0..args.len() {
                    self.pop(PARAM_REGISTERS[i]);
                }
                let name = self.retrieve_name(func)?;
                println!("\tcall {}", name.symbol);
            }
            ExprKind::Block(block) => {
                for stmt in &block.stmts {
                    self.codegen_stmt(stmt)?;
                }
            }
            ExprKind::If(cond, then, els) => {
                let label_id = self.get_new_label_id();
                self.codegen_expr(cond)?;
                println!("\tcmp rax, 0");
                if els.is_some() {
                    println!("\tje .Lelse{label_id}");
                } else {
                    println!("\tje .Lend{label_id}");
                }
                self.codegen_expr(then)?;

                if let Some(els) = els {
                    println!("\tjmp .Lend{label_id}");
                    println!(".Lelse{label_id}:");
                    self.codegen_expr(els)?;
                }
                println!(".Lend{label_id}:");
            }
            ExprKind::Struct(ident, fds) => {
                let _adt = self.ctx.lookup_adt_def(&ident.symbol).unwrap();
                for (_, fd) in fds {
                    // TODO: deal with order
                    self.codegen_expr(fd)?;
                    self.push();
                }
            }
        }

        // Extract the significant bits
        let ty = self.ctx.get_type(expr.id);
        match &*ty {
            Ty::Bool => {
                println!("\tmovzx rax, al");
            }
            Ty::I32 => {
                println!("\tmovsx rax, eax");
            }
            _ => (),
        }

        // FIXME: should remove this
        // When returning from functions which return (), make sure that rax stores the value 0
        if *ty == Ty::Unit {
            println!("\tmov rax, 0 # return unit ?");
        }

        Ok(())
    }

    fn codegen_addr_local_var(&mut self, ident: &'a Ident) -> Result<(), ()> {
        // Try to find ident in all locals
        if let Some(local) = self.get_current_frame().locals.get(&ident.symbol) {
            println!("#lval");
            println!("\tmov rax, rbp");
            println!("\tsub rax, {}", local.offset);
            Ok(())
        }
        // Try to find ident in all args
        else if let Some(arg) = self.get_current_frame().args.get(&ident.symbol) {
            println!("#lval");
            println!("\tmov rax, rbp");
            println!("\tsub rax, {}", arg.offset);
            Ok(())
        } else {
            eprintln!("Unknwon identifier: {}", ident.symbol);
            Err(())
        }
    }

    fn codegen_addr(&mut self, expr: &'a Expr) -> Result<(), ()> {
        match &expr.kind {
            ExprKind::Ident(ident) => {
                self.codegen_addr_local_var(ident)?;
                Ok(())
            }
            ExprKind::Index(array, index) => {
                let elem_ty_size = self.ctx.get_size(&self.ctx.get_type(expr.id));
                self.codegen_addr(array)?;
                self.push();
                self.codegen_expr(index)?;
                self.push();
                self.pop("rdi"); // rdi <- index
                println!("\tmov rax, {}", elem_ty_size); // rax <- size_of(size)
                println!("\tmul rdi"); // rax <- index * size_of(elem)
                self.pop("rdi"); // rdi <- base_addr
                println!("\tadd rax, rdi"); // rax <- base_addr + index * size_of(elem)
                Ok(())
            }
            ExprKind::Field(recv, fd) => {
                self.codegen_addr(recv)?;
                let adt = self
                    .ctx
                    .lookup_adt_def(self.ctx.get_type(recv.id).get_adt_name().unwrap())
                    .unwrap();
                let offs = self.ctx.get_field_offsett(adt, &fd.symbol).unwrap();
                println!("\tadd rax, {}", offs);
                Ok(())
            }
            _ => {
                eprintln!("ICE: Cannot codegen {:?} as lval", expr);
                Err(())
            }
        }
    }

    // FIXME: sync with `codegen_assign`
    fn codegen_assign_local_var(
        &mut self,
        name: &'a Ident,
        ty: &Ty,
        expr: &'a Expr,
    ) -> Result<(), ()> {
        if ty.is_adt() {
            let adt = self.ctx.lookup_adt_def(ty.get_adt_name().unwrap()).unwrap();
            let flatten_fields = self.ctx.flatten_struct(adt);
            self.codegen_expr(expr)?;
            for (_ty, ofs) in flatten_fields.iter().rev() {
                self.codegen_addr_local_var(name)?;
                println!("\tadd rax, {ofs}");
                println!("\tpop rdi");
                println!("\tmov [rax], rdi");
            }
        } else {
            self.codegen_addr_local_var(name)?;
            self.push();
            self.codegen_expr(expr)?;
            self.push();
            self.pop("rdi");
            self.pop("rax");
            println!("\tmov [rax], rdi");
        }
        Ok(())
    }

    // FIXME: sync with `codegen_assign_local_var`
    fn codegen_assign(&mut self, lhs: &'a Expr, rhs: &'a Expr) -> Result<(), ()> {
        println!("#assign");
        let ty = self.ctx.get_type(rhs.id);
        if ty.is_adt() {
            let adt = self.ctx.lookup_adt_def(ty.get_adt_name().unwrap()).unwrap();
            let flatten_fields = self.ctx.flatten_struct(adt);
            self.codegen_expr(rhs)?;
            for (_ty, ofs) in flatten_fields.iter().rev() {
                self.codegen_addr(lhs)?;
                println!("\tadd rax, {ofs}");
                println!("\tpop rdi");
                println!("\tmov [rax], rdi");
            }
        } else {
            self.codegen_addr(lhs)?;
            self.push();
            self.codegen_expr(rhs)?;
            self.push();
            self.pop("rdi");
            self.pop("rax");
            println!("\tmov [rax], rdi");
        }
        Ok(())
    }

    fn retrieve_name<'b>(&'b self, expr: &'b Expr) -> Result<&Ident, ()> {
        match &expr.kind {
            ExprKind::Ident(ident) => Ok(ident),
            _ => Err(()),
        }
    }

    fn push(&self) {
        println!("\tpush rax");
    }

    fn pop(&self, reg: &str) {
        println!("\tpop {}", reg);
    }

    fn clean_adt_on_stack(&self, adt: &AdtDef) {
        let size = self.ctx.get_adt_size(adt);
        // FIXME: correct?
        let pop_rax_time = size / 8;
        for _ in 0..pop_rax_time {
            self.pop("rax");
        }
    }
}
