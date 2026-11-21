//! The frozen numeric core.
//!
//! This is the first typed subset: every value is an `i64`, every function is
//! `(i64, ...) -> i64`, and control flow is `if`/`cond`. It is deliberately
//! smaller than `river-run`; lists, strings, records, JSON, and host calls are
//! rejected until the value representation is designed.

use std::collections::HashMap;
use std::fmt;

use anyhow::Result;
use river_eval::sexp::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Gt,
    Lt,
    Ge,
    Le,
    And,
    Or,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Not,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Lit(i64),
    Var(String),
    Let {
        name: String,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        else_: Box<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnOp,
        value: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub functions: Vec<Function>,
}

impl Program {
    pub fn function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|function| function.name == name)
    }
}

#[derive(Debug)]
pub struct CoreError(pub String);

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CoreError {}

pub fn parse(source: &str) -> Result<Program, CoreError> {
    let forms = river_eval::sexp::parse_all(source)
        .map_err(|error| CoreError(format!("parse: {error}")))?;
    let mut functions = Vec::new();

    for form in forms {
        let Some(list) = form.as_list() else {
            return Err(CoreError("top-level form is not a list".into()));
        };
        let Some(head) = list.first().and_then(Value::as_symbol) else {
            return Err(CoreError("top-level form has no name".into()));
        };
        match head {
            "defun" => functions.push(parse_defun(list)?),
            // Generated WIT metadata is not code.
            name if name.starts_with("define-") => {}
            other => {
                return Err(CoreError(format!(
                    "unsupported top-level form `{other}`; the numeric core is defun-only"
                )))
            }
        }
    }

    if functions.is_empty() {
        return Err(CoreError("program has no defun".into()));
    }
    let names: std::collections::HashSet<&str> = functions
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    for function in &functions {
        validate_calls(&function.body, &names)?;
    }
    Ok(Program { functions })
}

fn parse_defun(list: &[Value]) -> Result<Function, CoreError> {
    let name = list
        .get(1)
        .and_then(Value::as_symbol)
        .ok_or_else(|| CoreError("defun needs a name".into()))?
        .to_string();
    let params = list
        .get(2)
        .and_then(Value::as_list)
        .ok_or_else(|| CoreError(format!("defun `{name}` needs a parameter list")))?
        .iter()
        .map(|value| {
            value
                .as_symbol()
                .map(str::to_string)
                .ok_or_else(|| CoreError(format!("defun `{name}` has a non-symbol parameter")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let body = list
        .get(3)
        .ok_or_else(|| CoreError(format!("defun `{name}` needs a body")))?;
    if list.len() > 4 {
        return Err(CoreError(format!(
            "defun `{name}` has more than one body form; the numeric core is expression-only"
        )));
    }

    let mut env = HashMap::new();
    for param in &params {
        env.insert(param.clone(), ());
    }
    let body = lower(body, &mut env)?;
    Ok(Function { name, params, body })
}

fn validate_calls(
    expression: &Expr,
    names: &std::collections::HashSet<&str>,
) -> Result<(), CoreError> {
    match expression {
        Expr::Lit(_) | Expr::Var(_) => Ok(()),
        Expr::Let { value, body, .. } => {
            validate_calls(value, names)?;
            validate_calls(body, names)
        }
        Expr::If { cond, then, else_ } => {
            validate_calls(cond, names)?;
            validate_calls(then, names)?;
            validate_calls(else_, names)
        }
        Expr::Binary { left, right, .. } => {
            validate_calls(left, names)?;
            validate_calls(right, names)
        }
        Expr::Unary { value, .. } => validate_calls(value, names),
        Expr::Call { name, args } => {
            if !names.contains(name.as_str()) {
                return Err(CoreError(format!("unknown function `{name}`")));
            }
            for arg in args {
                validate_calls(arg, names)?;
            }
            Ok(())
        }
    }
}

fn lower(value: &Value, env: &mut HashMap<String, ()>) -> Result<Expr, CoreError> {
    match value {
        Value::Int(number) => Ok(Expr::Lit(*number)),
        Value::Bool(boolean) => Ok(Expr::Lit(i64::from(*boolean))),
        Value::Symbol(name) => {
            if env.contains_key(name) {
                Ok(Expr::Var(name.clone()))
            } else {
                Err(CoreError(format!("unbound symbol `{name}`")))
            }
        }
        Value::List(items) => {
            let head = items
                .first()
                .and_then(Value::as_symbol)
                .ok_or_else(|| CoreError("call has no head".into()))?;
            match head {
                "let" => lower_let(items, env),
                "if" => lower_if(items, env),
                "cond" => lower_cond(items, env),
                "and" | "or" | "+" | "-" | "*" | "/" | "=" | ">" | "<" | ">=" | "<=" => {
                    lower_binary(head, items, env)
                }
                "not" => {
                    let value = items
                        .get(1)
                        .ok_or_else(|| CoreError("not needs an argument".into()))?;
                    Ok(Expr::Unary {
                        op: UnOp::Not,
                        value: Box::new(lower(value, env)?),
                    })
                }
                name => {
                    if env.contains_key(name) {
                        return Err(CoreError(format!("`{name}` is a variable, not a function")));
                    }
                    let args = items[1..]
                        .iter()
                        .map(|arg| lower(arg, env))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(Expr::Call {
                        name: name.to_string(),
                        args,
                    })
                }
            }
        }
        other => Err(CoreError(format!(
            "`{other}` is outside the numeric core (i64 only)"
        ))),
    }
}

fn lower_let(items: &[Value], env: &mut HashMap<String, ()>) -> Result<Expr, CoreError> {
    let mut local_env = env.clone();
    let bindings = items
        .get(1)
        .and_then(Value::as_list)
        .ok_or_else(|| CoreError("let needs a binding list".into()))?;
    if bindings.is_empty() {
        return Err(CoreError("let needs at least one binding".into()));
    }
    let body = items
        .get(2)
        .ok_or_else(|| CoreError("let needs a body".into()))?;
    if items.len() > 3 {
        return Err(CoreError(
            "let has more than one body form; the numeric core is expression-only".into(),
        ));
    }

    let mut lowered_bindings = Vec::new();
    for binding in bindings {
        let pair = binding
            .as_list()
            .ok_or_else(|| CoreError("let binding is not a list".into()))?;
        let name = pair
            .first()
            .and_then(Value::as_symbol)
            .ok_or_else(|| CoreError("let binding needs a name".into()))?
            .to_string();
        let value = pair
            .get(1)
            .ok_or_else(|| CoreError("let binding needs a value".into()))?;
        let value = lower(value, &mut local_env)?;
        local_env.insert(name.clone(), ());
        lowered_bindings.push((name, value));
    }

    let mut lowered = lower(body, &mut local_env)?;
    for (name, value) in lowered_bindings.into_iter().rev() {
        lowered = Expr::Let {
            name,
            value: Box::new(value),
            body: Box::new(lowered),
        };
    }
    Ok(lowered)
}

fn lower_if(items: &[Value], env: &mut HashMap<String, ()>) -> Result<Expr, CoreError> {
    if items.len() != 4 {
        return Err(CoreError(
            "if needs a condition, a then branch, and an else branch".into(),
        ));
    }
    let mut then_env = env.clone();
    let mut else_env = env.clone();
    Ok(Expr::If {
        cond: Box::new(lower(&items[1], env)?),
        then: Box::new(lower(&items[2], &mut then_env)?),
        else_: Box::new(lower(&items[3], &mut else_env)?),
    })
}

fn lower_cond(items: &[Value], env: &mut HashMap<String, ()>) -> Result<Expr, CoreError> {
    let clauses = &items[1..];
    if clauses.is_empty() {
        return Err(CoreError("cond needs at least one clause".into()));
    }
    let mut lowered = None;
    for clause in clauses.iter().rev() {
        let list = clause
            .as_list()
            .ok_or_else(|| CoreError("cond clause is not a list".into()))?;
        let test = list
            .first()
            .ok_or_else(|| CoreError("cond clause has no test".into()))?;
        let body = list
            .get(1)
            .ok_or_else(|| CoreError("cond clause has no body".into()))?;
        if list.len() > 2 {
            return Err(CoreError("cond clause has more than one body form".into()));
        }
        let mut clause_env = env.clone();
        let body = lower(body, &mut clause_env)?;
        if test.as_symbol() == Some("else") {
            lowered = Some(body);
        } else {
            let cond = lower(test, &mut clause_env)?;
            let else_ = lowered
                .ok_or_else(|| CoreError("cond needs an else clause in the numeric core".into()))?;
            lowered = Some(Expr::If {
                cond: Box::new(cond),
                then: Box::new(body),
                else_: Box::new(else_),
            });
        }
    }
    lowered.ok_or_else(|| CoreError("cond needs an else clause".into()))
}

fn lower_binary(
    head: &str,
    items: &[Value],
    env: &mut HashMap<String, ()>,
) -> Result<Expr, CoreError> {
    if items.len() < 3 {
        return Err(CoreError(format!("`{head}` needs at least two arguments")));
    }
    let mut args = items[1..]
        .iter()
        .map(|value| lower(value, env))
        .collect::<Result<Vec<_>, _>>()?;
    let mut left = args.remove(0);
    for right in args {
        left = Expr::Binary {
            op: match head {
                "+" => BinOp::Add,
                "-" => BinOp::Sub,
                "*" => BinOp::Mul,
                "/" => BinOp::Div,
                "=" => BinOp::Eq,
                ">" => BinOp::Gt,
                "<" => BinOp::Lt,
                ">=" => BinOp::Ge,
                "<=" => BinOp::Le,
                "and" => BinOp::And,
                "or" => BinOp::Or,
                _ => unreachable!(),
            },
            left: Box::new(left),
            right: Box::new(right),
        };
    }
    Ok(left)
}
