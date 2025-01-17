use super::{
    frame::LocalKind,
    llvm::{LLReg, LLValue},
    Codegen,
};
use crate::{
    ast::{Expr, ExprKind, Path},
    backend_llvm::llvm::LLTy,
    resolve::Binding,
    span::Ident,
};
use std::rc::Rc;

impl<'gen, 'ctx> Codegen<'gen, 'ctx> {
    // expr: LLTY -> LLTY*
    pub fn gen_lval(&mut self, expr: &'gen Expr) -> Result<Rc<LLReg>, ()> {
        match &expr.kind {
            ExprKind::Path(path) => {
                let binding = self.ctx.resolve_path(path).unwrap();
                self.gen_binding_lval(&binding)
            }
            ExprKind::Index(arr, index) => {
                // TODO: move to another func
                let arr_ptr_reg = self.gen_lval(arr)?;
                let index_val = self.eval_expr(index)?;
                let new_reg = self.peek_frame_mut().get_fresh_reg();

                println!(
                    "\t{} = getelementptr {}, {}, i32 0, {}",
                    new_reg,
                    arr_ptr_reg.llty.peel_ptr().unwrap().to_string(),
                    arr_ptr_reg.to_string_with_type(),
                    index_val.to_string_with_type()
                );
                // `[N x elem_ty]*` => `elem_ty*`
                let ret_llty = LLTy::Ptr(
                    arr_ptr_reg
                        .llty
                        .peel_ptr()
                        .unwrap()
                        .get_element_type()
                        .unwrap(),
                );
                Ok(LLReg::new(new_reg, Rc::new(ret_llty)))
            }
            ExprKind::Field(strct, field) => {
                let struct_ptr = self.gen_lval(strct)?;
                self.gen_field_lval(&struct_ptr, field)
            }
            ExprKind::Struct(_, _) | ExprKind::Array(_) => {
                let ptr = self.peek_frame().get_ptr_to_temporary(expr.id).unwrap();
                self.initialize_memory_with_value(&ptr, expr)?;
                Ok(ptr)
            }
            ExprKind::If(cond, then, els) => {
                let res = self.gen_if_expr(cond, then, els)?;
                Ok(res.0)
            }
            ExprKind::Block(block) => {
                let llval = self.gen_block(block)?;
                if let LLValue::Reg(reg) = llval {
                    Ok(reg)
                } else {
                    panic!("ICE");
                }
            }
            ExprKind::Call(func, args) => {
                let must_be_void = self.gen_call_expr(expr.id, func, args)?;
                assert_eq!(*must_be_void.llty(), LLTy::Void);
                let temp = self.peek_frame().get_ptr_to_temporary(expr.id).unwrap();
                Ok(temp)
            }
            ExprKind::Binary(_,_,_) => {
                let e = self.eval_expr(expr)?; 
                let new_reg = self.peek_frame_mut().get_fresh_reg();
                let ty = self.ctx.get_type(expr.id); 
                let tmp = self.ty_to_llty(&ty); 
                let ty = tmp.to_string(); 
                println!("\t{} = alloca {}", new_reg, ty); 
                println!("\tstore {}, {}* {}", e.to_string_with_type(), ty, new_reg); 
                //println!("\n\n{:?} = {:?}", new_reg, e.to_string_with_type()); 
                
                Ok(LLReg::new(new_reg, Rc::new(LLTy::Ptr(Rc::new(tmp)))))
            }

            ExprKind::NumLit(_) => {
                let e = self.eval_expr(expr)?; 
                let new_reg = self.peek_frame_mut().get_fresh_reg(); 
                let ty = self.ctx.get_type(expr.id); 
                let ty = self.ty_to_llty(&ty).to_string(); 
                println!("\t{} = alloca {}", new_reg, ty); 
                println!("\tstore {}, {}* {}", e.to_string_with_type(), ty, new_reg); 
                
                Ok(LLReg::new(new_reg, Rc::new(LLTy::Ptr(Rc::new(LLTy::I32)))))
            }

            ExprKind::Deref(path) => {
                let e = self.load_path(path)?; 
                Ok(e)  
            }
            _ => todo!("{:?}", expr)
        }
    }

    // struct_ptr_reg: STRUCT*, s.field: FIELD_LLTY -> returns FIELD_LLTY*
    pub fn gen_field_lval(
        &mut self,
        struct_ptr_reg: &Rc<LLReg>,
        field: &'gen Ident,
    ) -> Result<Rc<LLReg>, ()> {
        let adt_name = struct_ptr_reg
            .llty
            .peel_ptr()
            .unwrap()
            .get_adt_cpath()
            .unwrap();
        let lladt = self.get_lladt(&adt_name).unwrap();
        let field_index = lladt.get_field_index(&field.symbol).unwrap();
        // `type { T1, T2, T3 }*` => `Tn*`
        let ret_llty = LLTy::Ptr(Rc::clone(&lladt.fields[field_index].1));

        let new_reg = self.peek_frame_mut().get_fresh_reg();
        println!(
            "\t{} = getelementptr {}, {}, i32 0, i32 {}",
            new_reg,
            struct_ptr_reg.llty.peel_ptr().unwrap().to_string(),
            struct_ptr_reg.to_string_with_type(),
            field_index
        );

        Ok(LLReg::new(new_reg, Rc::new(ret_llty)))
    }

    // ident: LLTY* (i.e. LocalKind::Ptr) -> LLTY*
    // ident: LLTY  (i.e. LocalKind::Val)  -> Err
    pub fn gen_binding_lval(&mut self, binding: &Binding) -> Result<Rc<LLReg>, ()> {
        let local = self.peek_frame().get_local(binding);
        match &local.kind {
            LocalKind::Value => Err(()),
            LocalKind::Ptr => Ok(Rc::clone(&local.reg)),
        }
    }

    /// ident is allocated on stack => load fromm its reg and returns the value
    /// ident is not allocated => returns its reg
    /// ident: LLTY -> returns LLTY*
    pub fn load_path(&mut self, path: &'gen Path) -> Result<Rc<LLReg>, ()> {
        let name = self.ctx.resolve_path(path).unwrap();
        let local = &self.peek_frame().get_local(&name);
        match &local.kind {
            LocalKind::Value => Ok(Rc::clone(&local.reg)),
            LocalKind::Ptr => self.load_ptr(&local.reg),
        }
    }

    // llty* -> llty
    pub fn load_ptr(&mut self, ptr: &Rc<LLReg>) -> Result<Rc<LLReg>, ()> {
        assert!(matches!(*ptr.llty, LLTy::Ptr(_)));
        let new_reg = self.peek_frame_mut().get_fresh_reg();
        let derefed_ty = ptr.llty.peel_ptr().unwrap();
        println!(
            "\t{} = load {}, {} {}",
            new_reg,
            derefed_ty.to_string(),
            ptr.llty.to_string(),
            ptr.name
        );
        Ok(LLReg::new(new_reg, derefed_ty))
    }

    /// initializer of let statement
    pub fn initialize_memory_with_value(
        &mut self,
        ptr: &Rc<LLReg>,
        init: &'gen Expr,
    ) -> Result<(), ()> {
        let init_llty = self.ty_to_llty(&self.ctx.get_type(init.id));
        assert_eq!(*ptr.llty.peel_ptr().unwrap(), init_llty);

        match &init.kind {
            ExprKind::Struct(path, fields) => {
                let binding = self.ctx.resolve_path(path).unwrap();
                let lladt = self.get_lladt(&binding.cpath).unwrap();
                for (field, fd_expr) in fields {
                    if lladt.get_field_index(&field.symbol).is_none() {
                        continue;
                    }
                    let fd_ptr = self.gen_field_lval(ptr, field)?;
                    self.initialize_memory_with_value(&fd_ptr, fd_expr)?
                }
            }
            ExprKind::Array(_) => {
                todo!()
            }
            _ => {
                if init_llty.eval_to_ptr() {
                    let e = self.eval_expr(init)?; 
                    println!("\tstore {}, {}", e.to_string_with_type(), ptr.to_string_with_type());  
                } else {
                    let init_val = self.eval_expr(init)?;
                    println!(
                        "\tstore {}, {}",
                        init_val.to_string_with_type(),
                        ptr.to_string_with_type()
                    );
                }
            }
        }
        Ok(())
    }

    // TODO: alignment?
    pub fn memcpy(&mut self, dist: &Rc<LLReg>, src: &Rc<LLReg>) {
        assert_eq!(dist.llty, src.llty);
        let target_llty = src.llty.peel_ptr().unwrap();
        let size = self.get_size(&target_llty);
        println!(
            "\tcall void @llvm.memcpy.p0i8.p0i8.i64(ptr {}, ptr {}, i64 {}, i1 false)",
            dist.name, src.name, size
        );
    }
}
