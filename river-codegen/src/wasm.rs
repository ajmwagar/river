//! Wasm code generation for the numeric core.

use std::collections::HashMap;

use anyhow::{bail, Result};
use wasm_encoder::{
    BlockType, CodeSection, ExportKind, ExportSection, Function as WasmFunction, FunctionSection,
    Instruction as I, Module, TypeSection, ValType,
};

use crate::core::{BinOp, Expr, Program, UnOp};

pub fn compile(program: &Program) -> Result<Vec<u8>> {
    let mut function_index = HashMap::new();
    for (index, function) in program.functions.iter().enumerate() {
        if function_index
            .insert(function.name.clone(), index as u32)
            .is_some()
        {
            bail!("duplicate function `{}`", function.name);
        }
    }

    let mut module = Module::new();

    let mut types = TypeSection::new();
    for function in &program.functions {
        types.ty().function(
            std::iter::repeat_n(ValType::I64, function.params.len()),
            [ValType::I64],
        );
    }
    module.section(&types);

    let mut functions = FunctionSection::new();
    for (index, _) in program.functions.iter().enumerate() {
        functions.function(index as u32);
    }
    module.section(&functions);

    let mut exports = ExportSection::new();
    for (index, function) in program.functions.iter().enumerate() {
        exports.export(&function.name, ExportKind::Func, index as u32);
    }
    module.section(&exports);

    let mut code = CodeSection::new();
    for function in &program.functions {
        let local_count = count_lets(&function.body);
        let locals = if local_count == 0 {
            Vec::new()
        } else {
            vec![(local_count, ValType::I64)]
        };
        let mut compiler = FunctionCompiler {
            program,
            function_index: &function_index,
            params: function
                .params
                .iter()
                .enumerate()
                .map(|(index, name)| (name.clone(), index as u32))
                .collect(),
            locals: function.params.len() as u32,
            body: WasmFunction::new(locals),
        };
        compiler.emit(&function.body)?;
        compiler.body.instruction(&I::End);
        code.function(&compiler.body);
    }
    module.section(&code);

    Ok(module.finish())
}

fn count_lets(expression: &Expr) -> u32 {
    match expression {
        Expr::Lit(_) | Expr::Var(_) => 0,
        Expr::Let { value, body, .. } => 1 + count_lets(value) + count_lets(body),
        Expr::If { cond, then, else_ } => count_lets(cond) + count_lets(then) + count_lets(else_),
        Expr::Binary { left, right, .. } => count_lets(left) + count_lets(right),
        Expr::Unary { value, .. } => count_lets(value),
        Expr::Call { args, .. } => args.iter().map(count_lets).sum(),
    }
}

struct FunctionCompiler<'a> {
    program: &'a Program,
    function_index: &'a HashMap<String, u32>,
    params: HashMap<String, u32>,
    locals: u32,
    body: WasmFunction,
}

impl FunctionCompiler<'_> {
    fn emit(&mut self, expression: &Expr) -> Result<()> {
        match expression {
            Expr::Lit(value) => {
                self.body.instruction(&I::I64Const(*value));
            }
            Expr::Var(name) => {
                let local = self
                    .params
                    .get(name)
                    .ok_or_else(|| anyhow::anyhow!("unknown local `{name}`"))?;
                self.body.instruction(&I::LocalGet(*local));
            }
            Expr::Let { name, value, body } => {
                self.emit(value)?;
                let local = self.locals;
                self.locals += 1;
                self.params.insert(name.clone(), local);
                self.body.instruction(&I::LocalSet(local));
                self.emit(body)?;
            }
            Expr::If { cond, then, else_ } => {
                self.emit(cond)?;
                self.body.instruction(&I::I32WrapI64);
                self.body
                    .instruction(&I::If(BlockType::Result(ValType::I64)));
                self.emit(then)?;
                self.body.instruction(&I::Else);
                self.emit(else_)?;
                self.body.instruction(&I::End);
            }
            Expr::Binary { op, left, right } => {
                self.emit(left)?;
                self.emit(right)?;
                match op {
                    BinOp::Add => {
                        self.body.instruction(&I::I64Add);
                    }
                    BinOp::Sub => {
                        self.body.instruction(&I::I64Sub);
                    }
                    BinOp::Mul => {
                        self.body.instruction(&I::I64Mul);
                    }
                    BinOp::Div => {
                        self.body.instruction(&I::I64DivS);
                    }
                    BinOp::Eq => {
                        self.body.instruction(&I::I64Eq);
                        self.body.instruction(&I::I64ExtendI32U);
                    }
                    BinOp::Gt => {
                        self.body.instruction(&I::I64GtS);
                        self.body.instruction(&I::I64ExtendI32U);
                    }
                    BinOp::Lt => {
                        self.body.instruction(&I::I64LtS);
                        self.body.instruction(&I::I64ExtendI32U);
                    }
                    BinOp::Ge => {
                        self.body.instruction(&I::I64GeS);
                        self.body.instruction(&I::I64ExtendI32U);
                    }
                    BinOp::Le => {
                        self.body.instruction(&I::I64LeS);
                        self.body.instruction(&I::I64ExtendI32U);
                    }
                    BinOp::And => {
                        self.body.instruction(&I::I64And);
                    }
                    BinOp::Or => {
                        self.body.instruction(&I::I64Or);
                    }
                }
            }
            Expr::Unary { op, value } => {
                self.emit(value)?;
                match op {
                    UnOp::Not => {
                        self.body.instruction(&I::I64Eqz);
                        self.body.instruction(&I::I64ExtendI32U);
                    }
                }
            }
            Expr::Call { name, args } => {
                let index = self
                    .function_index
                    .get(name)
                    .ok_or_else(|| anyhow::anyhow!("unknown function `{name}`"))?;
                let function = self
                    .program
                    .function(name)
                    .ok_or_else(|| anyhow::anyhow!("unknown function `{name}`"))?;
                if args.len() != function.params.len() {
                    bail!(
                        "`{name}` expects {} arguments, got {}",
                        function.params.len(),
                        args.len()
                    );
                }
                for arg in args {
                    self.emit(arg)?;
                }
                self.body.instruction(&I::Call(*index));
            }
        }
        Ok(())
    }
}
