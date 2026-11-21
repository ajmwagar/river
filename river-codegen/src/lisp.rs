//! Compile the McCarthy-style Lisp core to Wasm.
//!
//! This is the bootstrap target: symbols, cons cells, `quote`, `atom`, `eq`,
//! `car`, `cdr`, `cons`, `cond`, recursion, and function calls. Values are
//! tagged i64s:
//!
//! ```text
//! nil                0
//! symbol(id)         01 | id
//! cons(address)      10 | address
//! ```
//!
//! Cons cells live in linear memory. Quoted data is laid out statically.

use std::collections::{BTreeMap, HashMap};

use anyhow::{bail, Context, Result};
use river_eval::sexp::Value;
use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, DataSection, ExportKind, ExportSection,
    Function as WasmFunction, FunctionSection, GlobalSection, GlobalType, Instruction as I, MemArg,
    MemArgTarget, MemorySection, MemoryType, Module, TypeSection, ValType,
};

const BASE: i64 = 1024;
const ADDR_MASK: i64 = 0xFFFF_FFFF;
const SYMBOL_TAG: i64 = 1 << 62;
const CONS_TAG: i64 = 2 << 62;
const TAG_MASK: i64 = 3 << 62;

#[derive(Clone, Debug, PartialEq)]
pub enum Const {
    Nil,
    Symbol(String),
    List(Vec<Const>),
}

#[derive(Clone, Debug)]
enum Expr {
    Const(Const),
    Resolved(i64),
    Var(String),
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        else_: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Prim {
        prim: Prim,
        args: Vec<Expr>,
    },
}

#[derive(Clone, Copy, Debug)]
enum Prim {
    Atom,
    Eq,
    Car,
    Cdr,
    Cons,
}

#[derive(Clone, Debug)]
struct Function {
    name: String,
    params: Vec<String>,
    body: Expr,
}

#[derive(Clone, Debug)]
struct Program {
    functions: Vec<Function>,
}

pub struct Compiled {
    pub wasm: Vec<u8>,
    pub symbols: BTreeMap<String, i64>,
}

pub fn compile_source(source: &str) -> Result<Compiled> {
    compile(&parse(source)?)
}

fn parse(source: &str) -> Result<Program> {
    let forms = river_eval::sexp::parse_all(source).context("parse Lisp core")?;
    let mut functions = Vec::new();
    for form in forms {
        let Some(list) = form.as_list() else {
            bail!("top-level form is not a list");
        };
        let Some(head) = list.first().and_then(Value::as_symbol) else {
            bail!("top-level form has no name");
        };
        match head {
            "defun" => functions.push(parse_defun(list)?),
            name if name.starts_with("define-") => {}
            other => bail!("unsupported top-level form `{other}`"),
        }
    }
    if functions.is_empty() {
        bail!("program has no defun");
    }
    Ok(Program { functions })
}

fn parse_defun(list: &[Value]) -> Result<Function> {
    let name = list
        .get(1)
        .and_then(Value::as_symbol)
        .ok_or_else(|| anyhow::anyhow!("defun needs a name"))?
        .to_string();
    let params = list
        .get(2)
        .and_then(Value::as_list)
        .ok_or_else(|| anyhow::anyhow!("defun `{name}` needs a parameter list"))?
        .iter()
        .map(|value| {
            value
                .as_symbol()
                .map(str::to_string)
                .ok_or_else(|| anyhow::anyhow!("defun `{name}` has a non-symbol parameter"))
        })
        .collect::<Result<Vec<_>>>()?;
    let body = list
        .get(3)
        .ok_or_else(|| anyhow::anyhow!("defun `{name}` needs a body"))?;
    if list.len() > 4 {
        bail!("defun `{name}` has more than one body form");
    }
    let mut env = HashMap::new();
    for param in &params {
        env.insert(param.clone(), ());
    }
    Ok(Function {
        name: name.clone(),
        params,
        body: lower(body, &mut env).with_context(|| format!("defun `{name}`"))?,
    })
}

fn lower(value: &Value, env: &mut HashMap<String, ()>) -> Result<Expr> {
    match value {
        Value::Nil => Ok(Expr::Const(Const::Nil)),
        Value::Bool(boolean) => Ok(Expr::Const(if *boolean {
            Const::Symbol("t".to_string())
        } else {
            Const::Nil
        })),
        Value::Symbol(name) => {
            if env.contains_key(name) {
                Ok(Expr::Var(name.clone()))
            } else {
                bail!("unbound symbol `{name}`")
            }
        }
        Value::List(items) => {
            let head = items
                .first()
                .and_then(Value::as_symbol)
                .ok_or_else(|| anyhow::anyhow!("call has no head"))?;
            match head {
                "quote" => {
                    let value = items
                        .get(1)
                        .ok_or_else(|| anyhow::anyhow!("quote needs one argument"))?;
                    Ok(Expr::Const(const_value(value)?))
                }
                "cond" => lower_cond(items, env),
                "if" => lower_if(items, env),
                "atom" => lower_prim(Prim::Atom, items, env),
                "eq" => lower_prim(Prim::Eq, items, env),
                "car" => lower_prim(Prim::Car, items, env),
                "cdr" => lower_prim(Prim::Cdr, items, env),
                "cons" => lower_prim(Prim::Cons, items, env),
                name => {
                    let args = items[1..]
                        .iter()
                        .map(|arg| lower(arg, env))
                        .collect::<Result<Vec<_>>>()?;
                    Ok(Expr::Call {
                        name: name.to_string(),
                        args,
                    })
                }
            }
        }
        other => bail!("`{other}` is outside the Lisp core"),
    }
}

fn lower_prim(prim: Prim, items: &[Value], env: &mut HashMap<String, ()>) -> Result<Expr> {
    let expected = match prim {
        Prim::Cons | Prim::Eq => 2,
        _ => 1,
    };
    if items.len() != expected + 1 {
        bail!("primitive needs {expected} arguments");
    }
    Ok(Expr::Prim {
        prim,
        args: items[1..]
            .iter()
            .map(|arg| lower(arg, env))
            .collect::<Result<Vec<_>>>()?,
    })
}

fn lower_if(items: &[Value], env: &mut HashMap<String, ()>) -> Result<Expr> {
    if items.len() != 4 {
        bail!("if needs condition, then, and else");
    }
    let mut then_env = env.clone();
    let mut else_env = env.clone();
    Ok(Expr::If {
        cond: Box::new(lower(&items[1], env)?),
        then: Box::new(lower(&items[2], &mut then_env)?),
        else_: Box::new(lower(&items[3], &mut else_env)?),
    })
}

fn lower_cond(items: &[Value], env: &mut HashMap<String, ()>) -> Result<Expr> {
    let clauses = &items[1..];
    if clauses.is_empty() {
        bail!("cond needs clauses");
    }
    let mut lowered = None;
    for clause in clauses.iter().rev() {
        let list = clause
            .as_list()
            .ok_or_else(|| anyhow::anyhow!("cond clause is not a list"))?;
        let test = list
            .first()
            .ok_or_else(|| anyhow::anyhow!("cond clause has no test"))?;
        let body = list
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("cond clause has no body"))?;
        let mut clause_env = env.clone();
        let body = lower(body, &mut clause_env)?;
        if is_else_test(test) {
            lowered = Some(body);
        } else {
            let cond = lower(test, &mut clause_env)?;
            let else_ = lowered.unwrap_or(Expr::Const(Const::Nil));
            lowered = Some(Expr::If {
                cond: Box::new(cond),
                then: Box::new(body),
                else_: Box::new(else_),
            });
        }
    }
    Ok(lowered.unwrap_or(Expr::Const(Const::Nil)))
}

fn is_else_test(value: &Value) -> bool {
    if value.as_symbol() == Some("else") {
        return true;
    }
    matches!(
        value.as_list(),
        Some([quote, t])
            if quote.as_symbol() == Some("quote") && t.as_symbol() == Some("t")
    )
}

fn const_value(value: &Value) -> Result<Const> {
    match value {
        Value::Nil => Ok(Const::Nil),
        Value::Symbol(name) => Ok(Const::Symbol(name.clone())),
        Value::Bool(boolean) => Ok(if *boolean {
            Const::Symbol("t".to_string())
        } else {
            Const::Nil
        }),
        Value::List(items) => items
            .iter()
            .map(const_value)
            .collect::<Result<Vec<_>>>()
            .map(Const::List),
        other => bail!("`{other}` is not quoted Lisp data"),
    }
}

fn collect_symbols(expression: &Expr, symbols: &mut BTreeMap<String, i64>) {
    match expression {
        Expr::Const(value) => collect_const_symbols(value, symbols),
        Expr::Resolved(_) | Expr::Var(_) => {}
        Expr::If { cond, then, else_ } => {
            collect_symbols(cond, symbols);
            collect_symbols(then, symbols);
            collect_symbols(else_, symbols);
        }
        Expr::Call { args, .. } | Expr::Prim { args, .. } => {
            for arg in args {
                collect_symbols(arg, symbols);
            }
        }
    }
}

fn collect_const_symbols(value: &Const, symbols: &mut BTreeMap<String, i64>) {
    match value {
        Const::Nil => {}
        Const::Symbol(name) => {
            symbols.entry(name.clone()).or_insert(0);
        }
        Const::List(items) => {
            for item in items {
                collect_const_symbols(item, symbols);
            }
        }
    }
}

struct Resolver<'a> {
    symbols: &'a BTreeMap<String, i64>,
    data: Vec<u8>,
}

impl Resolver<'_> {
    fn resolve(&mut self, expression: Expr) -> Result<Expr> {
        Ok(match expression {
            Expr::Const(value) => Expr::Resolved(self.resolve_const(&value)?),
            Expr::Resolved(value) => Expr::Resolved(value),
            Expr::Var(name) => Expr::Var(name),
            Expr::If { cond, then, else_ } => Expr::If {
                cond: Box::new(self.resolve(*cond)?),
                then: Box::new(self.resolve(*then)?),
                else_: Box::new(self.resolve(*else_)?),
            },
            Expr::Call { name, args } => Expr::Call {
                name,
                args: args
                    .into_iter()
                    .map(|arg| self.resolve(arg))
                    .collect::<Result<Vec<_>>>()?,
            },
            Expr::Prim { prim, args } => Expr::Prim {
                prim,
                args: args
                    .into_iter()
                    .map(|arg| self.resolve(arg))
                    .collect::<Result<Vec<_>>>()?,
            },
        })
    }

    fn resolve_const(&mut self, value: &Const) -> Result<i64> {
        match value {
            Const::Nil => Ok(0),
            Const::Symbol(name) => self
                .symbols
                .get(name)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("missing symbol `{name}`")),
            Const::List(items) => {
                let mut tail: i64 = 0;
                for item in items.iter().rev() {
                    let car = self.resolve_const(item)?;
                    let address = BASE + self.data.len() as i64;
                    self.data.extend(car.to_le_bytes());
                    self.data.extend(tail.to_le_bytes());
                    tail = CONS_TAG | address;
                }
                Ok(tail)
            }
        }
    }
}

fn compile(program: &Program) -> Result<Compiled> {
    let mut symbols = BTreeMap::new();
    symbols.insert("t".to_string(), 0);
    for function in &program.functions {
        collect_symbols(&function.body, &mut symbols);
    }
    let mut next = 2;
    for (name, value) in symbols.iter_mut() {
        if name == "t" {
            *value = SYMBOL_TAG | 1;
        } else {
            *value = SYMBOL_TAG | next;
            next += 1;
        }
    }

    let mut resolver = Resolver {
        symbols: &symbols,
        data: Vec::new(),
    };
    let mut functions = Vec::new();
    for function in &program.functions {
        functions.push(Function {
            name: function.name.clone(),
            params: function.params.clone(),
            body: resolver.resolve(function.body.clone())?,
        });
    }
    let data = resolver.data;
    let heap = BASE + data.len() as i64;
    let pages = ((heap + 65535) / 65536).max(1) as u64;

    let index: HashMap<String, u32> = functions
        .iter()
        .enumerate()
        .map(|(index, function)| (function.name.clone(), index as u32))
        .collect();

    let mut module = Module::new();

    let mut types = TypeSection::new();
    for function in &functions {
        types.ty().function(
            std::iter::repeat_n(ValType::I64, function.params.len()),
            [ValType::I64],
        );
    }
    module.section(&types);

    let mut wasm_functions = FunctionSection::new();
    for (index, _) in functions.iter().enumerate() {
        wasm_functions.function(index as u32);
    }
    module.section(&wasm_functions);

    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: pages,
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    module.section(&memories);

    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I64,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i64_const(heap),
    );
    module.section(&globals);

    let mut exports = ExportSection::new();
    for (index, function) in functions.iter().enumerate() {
        exports.export(&function.name, ExportKind::Func, index as u32);
    }
    module.section(&exports);

    let mut code = CodeSection::new();
    for function in &functions {
        let temp_count = count_temps(&function.body);
        let mut compiler = FunctionCompiler {
            functions: &functions,
            index: &index,
            locals: function
                .params
                .iter()
                .enumerate()
                .map(|(index, name)| (name.clone(), index as u32))
                .collect(),
            next_temp: function.params.len() as u32,
            body: WasmFunction::new(if temp_count == 0 {
                Vec::new()
            } else {
                vec![(temp_count, ValType::I64)]
            }),
        };
        compiler.emit(&function.body)?;
        compiler.body.instruction(&I::End);
        code.function(&compiler.body);
    }
    module.section(&code);

    if !data.is_empty() {
        let mut data_section = DataSection::new();
        data_section.active(0, &ConstExpr::i32_const(BASE as i32), data);
        module.section(&data_section);
    }

    Ok(Compiled {
        wasm: module.finish(),
        symbols,
    })
}

fn count_temps(expression: &Expr) -> u32 {
    match expression {
        Expr::Const(_) | Expr::Resolved(_) | Expr::Var(_) => 0,
        Expr::If { cond, then, else_ } => {
            count_temps(cond) + count_temps(then) + count_temps(else_)
        }
        Expr::Call { args, .. } => args.iter().map(count_temps).sum(),
        Expr::Prim { prim, args } => {
            let own = match prim {
                Prim::Cons => 3,
                Prim::Car | Prim::Cdr => 2,
                Prim::Atom | Prim::Eq => 0,
            };
            own + args.iter().map(count_temps).sum::<u32>()
        }
    }
}

struct FunctionCompiler<'a> {
    functions: &'a [Function],
    index: &'a HashMap<String, u32>,
    locals: HashMap<String, u32>,
    next_temp: u32,
    body: WasmFunction,
}

impl FunctionCompiler<'_> {
    fn emit(&mut self, expression: &Expr) -> Result<()> {
        match expression {
            Expr::Resolved(value) => {
                self.body.instruction(&I::I64Const(*value));
            }
            Expr::Const(value) => bail!("unresolved quote `{value:?}`"),
            Expr::Var(name) => {
                let local = self
                    .locals
                    .get(name)
                    .ok_or_else(|| anyhow::anyhow!("unknown local `{name}`"))?;
                self.body.instruction(&I::LocalGet(*local));
            }
            Expr::If { cond, then, else_ } => {
                self.emit(cond)?;
                self.body.instruction(&I::I64Eqz);
                self.body.instruction(&I::I32Eqz);
                self.body
                    .instruction(&I::If(BlockType::Result(ValType::I64)));
                self.emit(then)?;
                self.body.instruction(&I::Else);
                self.emit(else_)?;
                self.body.instruction(&I::End);
            }
            Expr::Call { name, args } => {
                let index = self
                    .index
                    .get(name)
                    .ok_or_else(|| anyhow::anyhow!("unknown function `{name}`"))?;
                let function = self
                    .functions
                    .iter()
                    .find(|function| function.name == *name)
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
            Expr::Prim { prim, args } => self.emit_prim(*prim, args)?,
        }
        Ok(())
    }

    fn emit_prim(&mut self, prim: Prim, args: &[Expr]) -> Result<()> {
        match prim {
            Prim::Atom => {
                self.emit(&args[0])?;
                self.body.instruction(&I::I64Const(TAG_MASK));
                self.body.instruction(&I::I64And);
                self.body.instruction(&I::I64Const(CONS_TAG));
                self.body.instruction(&I::I64Ne);
                self.body
                    .instruction(&I::If(BlockType::Result(ValType::I64)));
                self.body.instruction(&I::I64Const(self.symbol("t")?));
                self.body.instruction(&I::Else);
                self.body.instruction(&I::I64Const(0));
                self.body.instruction(&I::End);
            }
            Prim::Eq => {
                self.emit(&args[0])?;
                self.emit(&args[1])?;
                self.body.instruction(&I::I64Eq);
                self.body
                    .instruction(&I::If(BlockType::Result(ValType::I64)));
                self.body.instruction(&I::I64Const(self.symbol("t")?));
                self.body.instruction(&I::Else);
                self.body.instruction(&I::I64Const(0));
                self.body.instruction(&I::End);
            }
            Prim::Car | Prim::Cdr => {
                let value = self.temp();
                self.emit(&args[0])?;
                self.body.instruction(&I::LocalSet(value));
                self.body.instruction(&I::LocalGet(value));
                self.body.instruction(&I::I64Const(TAG_MASK));
                self.body.instruction(&I::I64And);
                self.body.instruction(&I::I64Const(CONS_TAG));
                self.body.instruction(&I::I64Eq);
                self.body
                    .instruction(&I::If(BlockType::Result(ValType::I64)));
                self.body.instruction(&I::LocalGet(value));
                self.body.instruction(&I::I64Const(ADDR_MASK));
                self.body.instruction(&I::I64And);
                self.body.instruction(&I::I32WrapI64);
                self.body
                    .instruction(&I::I64Load(mem_arg(if matches!(prim, Prim::Car) {
                        0
                    } else {
                        8
                    })));
                self.body.instruction(&I::Else);
                self.body.instruction(&I::I64Const(0));
                self.body.instruction(&I::End);
            }
            Prim::Cons => {
                let car = self.temp();
                let cdr = self.temp();
                let address = self.temp();
                self.emit(&args[0])?;
                self.body.instruction(&I::LocalSet(car));
                self.emit(&args[1])?;
                self.body.instruction(&I::LocalSet(cdr));
                self.body.instruction(&I::GlobalGet(0));
                self.body.instruction(&I::LocalSet(address));
                self.body.instruction(&I::LocalGet(address));
                self.body.instruction(&I::I32WrapI64);
                self.body.instruction(&I::LocalGet(car));
                self.body.instruction(&I::I64Store(mem_arg(0)));
                self.body.instruction(&I::LocalGet(address));
                self.body.instruction(&I::I32WrapI64);
                self.body.instruction(&I::LocalGet(cdr));
                self.body.instruction(&I::I64Store(mem_arg(8)));
                self.body.instruction(&I::GlobalGet(0));
                self.body.instruction(&I::I64Const(16));
                self.body.instruction(&I::I64Add);
                self.body.instruction(&I::GlobalSet(0));
                self.body.instruction(&I::LocalGet(address));
                self.body.instruction(&I::I64Const(CONS_TAG));
                self.body.instruction(&I::I64Or);
            }
        }
        Ok(())
    }

    fn temp(&mut self) -> u32 {
        let local = self.next_temp;
        self.next_temp += 1;
        local
    }

    fn symbol(&self, name: &str) -> Result<i64> {
        // The resolved-quote symbol map is captured by the program; `t` is
        // always present, and primitives only need `t`.
        let _ = name;
        Ok(SYMBOL_TAG | 1)
    }
}

fn mem_arg(offset: u64) -> MemArg {
    MemArg {
        offset,
        align: 3,
        target: MemArgTarget::Memory(0),
        ordering: None,
    }
}
