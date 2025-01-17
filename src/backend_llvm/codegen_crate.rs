use std::rc::Rc;
use super::{Codegen, LLValue};
use crate::{
    ast::{Block, Crate, ExternBlock, Func, Item, ItemKind, LetStmt, Stmt, StmtKind},
    backend_llvm::{
        frame::{compute_frame, LocalKind},
        llvm::{LLReg, LLTy},
        LLImm,
    },
};

impl<'gen, 'ctx> Codegen<'gen, 'ctx> {
    pub fn gen_crate(&mut self, krate: &'gen Crate) -> Result<(), ()> {
        for item in &krate.items {
            self.gen_item(item)?;
        }
        Ok(())
    }

    pub fn gen_item(&mut self, item: &'gen Item) -> Result<(), ()> {
        match &item.kind {
            ItemKind::Impl(implements) => {
                for func in &implements.methods {
                    self.gen_func(func)?;
                }
            }
            ItemKind::Func(func) => {
                self.gen_func(func)?;
            }
            ItemKind::Struct(_) => (),
            ItemKind::ExternBlock(ext_block) => self.gen_external_block(ext_block)?,
            ItemKind::Mod(module) => {
                for inner_item in &module.items {
                    self.gen_item(inner_item)?;
                }
            }
            ItemKind::TypeAlias(alias) => {
                let binding = self.ctx.get_binding(&alias.name).unwrap(); 
                println!("Binding: {:?}", binding); 
            }
        }
        Ok(())
    }

    pub fn gen_external_block(&mut self, ext_block: &'gen ExternBlock) -> Result<(), ()> {
        for func in &ext_block.funcs {
            self.gen_func(func)?;
        }
        Ok(())
    }

    fn gen_func(&mut self, func: &'gen Func) -> Result<(), ()> {
        // do not generate code for the func if it does not have its body
        if func.body.is_none() {
            print!("declare ")
        } else {
            print!("define ")
        }

        // collect information about all variables including parameters
        let frame = compute_frame(self, func);
        self.push_frame(frame);

        let fn_name_binding = self.ctx.get_binding(&func.name).unwrap();
        let (_param_tys, ret_ty) = self
            .ctx
            .lookup_name_type(&fn_name_binding)
            .unwrap()
            .get_func_type()
            .unwrap();

        let ret_llty = Rc::new(self.ty_to_llty(&ret_ty));
        let actual_ret_llty = if ret_llty.is_void() || ret_llty.eval_to_ptr() {
            &LLTy::Void
        } else {
            &ret_llty
        };

        print!(
            "{} @{}(",
            actual_ret_llty.to_string(),
            fn_name_binding.cpath.demangle()
        );

        // sret
        if ret_llty.eval_to_ptr() {
            let sret_reg_name = self.peek_frame_mut().get_fresh_reg();
            print!("ptr sret({}) {}", ret_llty.to_string(), sret_reg_name);
            self.peek_frame_mut().set_sret_reg(LLReg::new(
                sret_reg_name,
                Rc::new(LLTy::Ptr(Rc::clone(&ret_llty))),
            ));
            if !func.params.is_empty() {
                print!(",");
            }
        }

        // parameters
        let mut it = self
            .peek_frame()
            .get_locals()
            .iter()
            .filter(|(bind, l)| bind.kind.is_param() && !l.reg.llty.is_void())
            .peekable();
        while let Some((_, local)) = it.next() {
            print!("{}", local.reg.to_string_with_type());
            if it.peek().is_some() {
                print!(", ");
            }
        }
        
        if func.variadic {
            print!(", ..."); 
        }
        print!(")");

        let Some(body) = &func.body else{
            println!();
            return Ok(());
        };

        println!(" {{");
        println!("start:");

        // allocate local variables
        for (bind, local) in self.peek_frame().get_locals() {
            if bind.kind.is_let() && !local.reg.llty.is_void() {
                assert!(local.kind == LocalKind::Ptr);
                println!(
                    "\t{} = alloca {}",
                    local.reg.name,
                    local.reg.llty.peel_ptr().unwrap().to_string()
                );
            }
        }

        // allocate temporary variables
        for reg in self.peek_frame().get_ptrs_to_temporary().values() {
            println!(
                "\t{} = alloca {}",
                reg.name,
                reg.llty.peel_ptr().unwrap().to_string()
            );
        }

        let body_val = self.gen_block(body)?;

        if !self.ctx.get_type(body.id).is_never() {
            if ret_llty.eval_to_ptr() {
                let LLValue::Reg(body_val_reg) = body_val else {
                    panic!("ICE");
                };
                self.memcpy(&self.peek_frame().get_sret_reg().unwrap(), &body_val_reg);
                println!("\tret void");
            } else {
                println!("\tret {}", body_val.to_string_with_type());
            }
        }

        println!("}}");
        println!();

        self.pop_frame();

        Ok(())
    }

    pub fn gen_block(&mut self, block: &'gen Block) -> Result<LLValue, ()> {
        let mut last_stmt_val = None;
        for stmt in &block.stmts {
            last_stmt_val = Some(self.gen_stmt(stmt)?);
        }
        let ret = last_stmt_val.unwrap_or(LLValue::Imm(LLImm::Void));
        Ok(ret)
    }

    fn gen_stmt(&mut self, stmt: &'gen Stmt) -> Result<LLValue, ()> {
        // println!("; Starts stmt `{}`", stmt.span.to_snippet());
        let val = match &stmt.kind {
            StmtKind::Semi(expr) => {
                self.eval_expr(expr)?;
                LLValue::Imm(LLImm::Void)
            }
            StmtKind::Expr(expr) => self.eval_expr(expr)?,
            StmtKind::Let(LetStmt { ident, ty: _, init, mutable: _ }) => {
                let binding = self.ctx.get_binding(ident).unwrap();
                let local = self.peek_frame().get_local(&binding);

                if let Some(init) = init && local.kind == LocalKind::Ptr {
                    let ptr = self.gen_binding_lval(&binding).unwrap();
                    // assign initializer
                    self.initialize_memory_with_value(&ptr, init)?;
                }
                LLValue::Imm(LLImm::Void)
            }
        };
        // println!("; Finished stmt `{}`", stmt.span.to_snippet());
        Ok(val)
    }
}
