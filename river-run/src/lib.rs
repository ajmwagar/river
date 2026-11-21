//! A small closed evaluator for River S-expressions.
//!
//! This is deliberately not a general Lisp. It evaluates the forms River emits:
//! `defun`, `let`, `if`, `cond`, `and`/`or`/`not`, a few list/field helpers, and
//! exactly one host boundary: `(host-call interface function args)`.
//!
//! The generated WIT bindings define `defun` stubs over `host-call`; the host
//! owns grants and dispatch.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use river_eval::sexp::{parse_all, Value};

pub mod records_host;
pub use records_host::SoilRecordsHost;

#[derive(Clone)]
struct Function {
    params: Vec<String>,
    body: Vec<Value>,
}

pub struct Runtime {
    functions: HashMap<String, Function>,
    host: Box<dyn Host>,
}

pub trait Host {
    fn granted(&self, interface: &str, function: &str) -> bool;
    fn call(
        &mut self,
        interface: &str,
        function: &str,
        args: &[Value],
    ) -> std::result::Result<Value, String>;
}

impl Runtime {
    pub fn new(host: Box<dyn Host>) -> Self {
        Self {
            functions: HashMap::new(),
            host,
        }
    }

    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    pub fn load_file(&mut self, path: impl AsRef<Path>) -> Result<Value> {
        let path = path.as_ref();
        let source =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        self.load(&source)
            .with_context(|| format!("evaluate {}", path.display()))
    }

    pub fn load(&mut self, source: &str) -> Result<Value> {
        let forms = parse_all(source)?;
        let mut last = Value::Nil;

        for form in forms {
            if let Some(list) = form.as_list() {
                let head = list.first().and_then(Value::as_symbol);
                if head == Some("defun") {
                    let (name, function) = parse_defun(list)?;
                    self.functions.insert(name, function);
                    continue;
                }
                if head.is_some_and(|name| name.starts_with("define-")) {
                    continue;
                }
            }

            let mut env = HashMap::new();
            last = self.eval(&form, &mut env)?;
        }

        Ok(last)
    }

    pub fn call(&mut self, name: &str, args: &[Value]) -> Result<Value> {
        let function = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("unknown function `{name}`"))?;
        self.call_function(&function, args.to_vec())
    }

    fn call_function(&mut self, function: &Function, args: Vec<Value>) -> Result<Value> {
        if args.len() != function.params.len() {
            bail!(
                "function expects {} arguments, got {}",
                function.params.len(),
                args.len()
            );
        }

        let mut env = HashMap::new();
        for (name, value) in function.params.iter().zip(args) {
            env.insert(name.clone(), value);
        }
        self.eval_body(&function.body, &mut env)
    }

    fn eval_body(&mut self, body: &[Value], env: &mut HashMap<String, Value>) -> Result<Value> {
        let mut last = Value::Nil;
        for expression in body {
            last = self.eval(expression, env)?;
        }
        Ok(last)
    }

    fn eval(&mut self, expression: &Value, env: &mut HashMap<String, Value>) -> Result<Value> {
        match expression {
            Value::Nil
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::String(_)
            | Value::Keyword(_) => Ok(expression.clone()),
            Value::Symbol(name) => env
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow!("unbound symbol `{name}`")),
            Value::List(items) => {
                let head = items
                    .first()
                    .and_then(Value::as_symbol)
                    .ok_or_else(|| anyhow!("empty or invalid call `{expression}`"))?;

                match head {
                    "quote" => Ok(items
                        .get(1)
                        .cloned()
                        .ok_or_else(|| anyhow!("quote needs one argument"))?),
                    "if" => {
                        let test = self.eval(require(items, 1, "if")?, env)?;
                        let branch = if truthy(&test) { 2 } else { 3 };
                        match items.get(branch) {
                            Some(value) => self.eval(value, env),
                            None => Ok(Value::Nil),
                        }
                    }
                    "cond" => self.eval_cond(&items[1..], env),
                    "let" => self.eval_let(&items[1..], env),
                    "and" => {
                        let mut last = Value::Bool(true);
                        for item in &items[1..] {
                            last = self.eval(item, env)?;
                            if !truthy(&last) {
                                return Ok(last);
                            }
                        }
                        Ok(last)
                    }
                    "or" => {
                        for item in &items[1..] {
                            let value = self.eval(item, env)?;
                            if truthy(&value) {
                                return Ok(value);
                            }
                        }
                        Ok(Value::Bool(false))
                    }
                    "not" => Ok(Value::Bool(!truthy(
                        &self.eval(require(items, 1, "not")?, env)?,
                    ))),
                    "begin" | "do" => self.eval_body(&items[1..], env),
                    "host-call" => self.eval_host_call(items, env),
                    "list" => Ok(Value::List(self.eval_args(&items[1..], env)?)),
                    "field" | "get" => {
                        let record = self.eval(require(items, 1, "field")?, env)?;
                        let name = self.eval(require(items, 2, "field")?, env)?;
                        Ok(field_value(&record, &name))
                    }
                    "length" => {
                        let value = self.eval(require(items, 1, "length")?, env)?;
                        Ok(match &value {
                            Value::List(values) => Value::Int(values.len() as i64),
                            Value::String(value) => Value::Int(value.chars().count() as i64),
                            _ => bail!("length expects a list or string"),
                        })
                    }
                    "str" => Ok(Value::String(
                        self.eval_args(&items[1..], env)?
                            .iter()
                            .map(display_text)
                            .collect::<Vec<_>>()
                            .join(""),
                    )),
                    "println" => {
                        let values = self.eval_args(&items[1..], env)?;
                        println!(
                            "{}",
                            values
                                .iter()
                                .map(display_text)
                                .collect::<Vec<_>>()
                                .join(" ")
                        );
                        Ok(Value::Nil)
                    }
                    "ok?" => Ok(Value::Bool(is_tag(
                        &self.eval(require(items, 1, "ok?")?, env)?,
                        "ok",
                    ))),
                    "err?" => Ok(Value::Bool(is_tag(
                        &self.eval(require(items, 1, "err?")?, env)?,
                        "err",
                    ))),
                    "value" => {
                        let value = self.eval(require(items, 1, "value")?, env)?;
                        Ok(match value.as_list() {
                            Some([_, value]) => value.clone(),
                            _ => bail!("value expects a two-element tagged list"),
                        })
                    }
                    "nil?" => Ok(Value::Bool(matches!(
                        self.eval(require(items, 1, "nil?")?, env)?,
                        Value::Nil
                    ))),
                    "first" => {
                        let value = self.eval(require(items, 1, "first")?, env)?;
                        Ok(value
                            .as_list()
                            .and_then(|values| values.first())
                            .cloned()
                            .unwrap_or(Value::Nil))
                    }
                    "rest" => {
                        let value = self.eval(require(items, 1, "rest")?, env)?;
                        Ok(value
                            .as_list()
                            .map(|values| Value::List(values.iter().skip(1).cloned().collect()))
                            .unwrap_or(Value::Nil))
                    }
                    "json-decode" => {
                        let value = self.eval(require(items, 1, "json-decode")?, env)?;
                        let bytes = value_bytes(&value)?;
                        let json: serde_json::Value = serde_json::from_slice(&bytes)
                            .with_context(|| "invalid JSON in json-decode")?;
                        Ok(json_to_value(&json))
                    }
                    "json-encode" => {
                        let value = self.eval(require(items, 1, "json-encode")?, env)?;
                        Ok(Value::String(
                            serde_json::to_string(&value_to_json(&value))
                                .with_context(|| "json-encode failed")?,
                        ))
                    }
                    "+" | "-" | "*" | "/" => self.eval_arithmetic(head, &items[1..], env),
                    "=" | ">" | "<" | ">=" | "<=" => self.eval_comparison(head, &items[1..], env),
                    name if self.functions.contains_key(name) => {
                        let function = self.functions.get(name).cloned().unwrap();
                        let args = self.eval_args(&items[1..], env)?;
                        self.call_function(&function, args)
                    }
                    name => bail!("unknown form or function `{name}`"),
                }
            }
        }
    }

    fn eval_args(
        &mut self,
        expressions: &[Value],
        env: &mut HashMap<String, Value>,
    ) -> Result<Vec<Value>> {
        expressions
            .iter()
            .map(|expression| self.eval(expression, env))
            .collect()
    }

    fn eval_cond(&mut self, clauses: &[Value], env: &mut HashMap<String, Value>) -> Result<Value> {
        for clause in clauses {
            let list = clause
                .as_list()
                .ok_or_else(|| anyhow!("cond clause is not a list"))?;
            let test = list
                .first()
                .ok_or_else(|| anyhow!("cond clause has no test"))?;
            let matched = match test {
                Value::Symbol(name) if name == "else" => true,
                _ => truthy(&self.eval(test, env)?),
            };
            if matched {
                return self.eval_body(&list[1..], env);
            }
        }
        Ok(Value::Nil)
    }

    fn eval_let(&mut self, args: &[Value], env: &mut HashMap<String, Value>) -> Result<Value> {
        let bindings = args
            .first()
            .and_then(Value::as_list)
            .ok_or_else(|| anyhow!("let needs a binding list"))?;
        let mut next = env.clone();

        for binding in bindings {
            let pair = binding
                .as_list()
                .ok_or_else(|| anyhow!("let binding is not a list"))?;
            let name = pair
                .first()
                .and_then(Value::as_symbol)
                .ok_or_else(|| anyhow!("let binding needs a name"))?;
            let value = self.eval(
                pair.get(1)
                    .ok_or_else(|| anyhow!("let binding needs a value"))?,
                env,
            )?;
            next.insert(name.to_string(), value);
        }

        self.eval_body(&args[1..], &mut next)
    }

    fn eval_host_call(
        &mut self,
        items: &[Value],
        env: &mut HashMap<String, Value>,
    ) -> Result<Value> {
        let interface = self.eval(require(items, 1, "host-call")?, env)?;
        let function = self.eval(require(items, 2, "host-call")?, env)?;
        let arguments = match items.get(3) {
            Some(expression) => self.eval(expression, env)?,
            None => Value::Nil,
        };

        let interface = value_text(&interface);
        let function = value_text(&function);
        let args = match arguments {
            Value::Nil => Vec::new(),
            Value::List(values) => values,
            _ => bail!("host-call arguments must be a list or nil"),
        };

        if !self.host.granted(&interface, &function) {
            bail!("denied: {interface}/{function} is not granted to this caller");
        }

        self.host
            .call(&interface, &function, &args)
            .map_err(|error| anyhow!("host error: {error}"))
    }

    fn eval_arithmetic(
        &mut self,
        operator: &str,
        expressions: &[Value],
        env: &mut HashMap<String, Value>,
    ) -> Result<Value> {
        let values = self.eval_args(expressions, env)?;
        let numbers = values
            .iter()
            .map(|value| {
                value
                    .as_f64()
                    .ok_or_else(|| anyhow!("{operator} expects numbers"))
            })
            .collect::<Result<Vec<_>>>()?;

        if operator == "/" {
            if numbers.len() < 2 {
                bail!("/ needs at least two arguments");
            }
            let mut result = numbers[0];
            for number in &numbers[1..] {
                if *number == 0.0 {
                    bail!("division by zero");
                }
                result /= number;
            }
            return Ok(Value::Float(result));
        }

        let mut result = numbers.first().copied().unwrap_or(0.0);
        for number in numbers.iter().skip(1) {
            match operator {
                "+" => result += *number,
                "-" => result -= *number,
                "*" => result *= *number,
                _ => unreachable!(),
            }
        }

        let all_ints = values.iter().all(|value| matches!(value, Value::Int(_)));
        if all_ints {
            Ok(Value::Int(result as i64))
        } else {
            Ok(Value::Float(result))
        }
    }

    fn eval_comparison(
        &mut self,
        operator: &str,
        expressions: &[Value],
        env: &mut HashMap<String, Value>,
    ) -> Result<Value> {
        if expressions.len() != 2 {
            bail!("{operator} needs two arguments");
        }
        let left = self.eval(&expressions[0], env)?;
        let right = self.eval(&expressions[1], env)?;

        let result = match (left.as_f64(), right.as_f64()) {
            (Some(left), Some(right)) => match operator {
                "=" => left == right,
                ">" => left > right,
                "<" => left < right,
                ">=" => left >= right,
                "<=" => left <= right,
                _ => unreachable!(),
            },
            _ => match operator {
                "=" => left == right,
                _ => bail!("{operator} expects numbers"),
            },
        };

        Ok(Value::Bool(result))
    }
}

fn value_bytes(value: &Value) -> Result<Vec<u8>> {
    match value {
        Value::List(items) => items
            .iter()
            .map(|item| match item {
                Value::Int(byte) if (0..=255).contains(byte) => Ok(*byte as u8),
                _ => bail!("body must be a list of bytes"),
            })
            .collect(),
        Value::String(text) => Ok(text.as_bytes().to_vec()),
        _ => bail!("body must be a list of bytes or a string"),
    }
}

fn json_to_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(value) => Value::Bool(*value),
        serde_json::Value::Number(number) => number
            .as_i64()
            .map(Value::Int)
            .or_else(|| number.as_f64().map(Value::Float))
            .unwrap_or(Value::Nil),
        serde_json::Value::String(text) => Value::String(text.clone()),
        serde_json::Value::Array(items) => Value::List(items.iter().map(json_to_value).collect()),
        serde_json::Value::Object(object) => Value::List(
            object
                .iter()
                .map(|(key, value)| pair(key, json_to_value(value)))
                .collect(),
        ),
    }
}

fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Nil => serde_json::Value::Null,
        Value::Bool(value) => serde_json::Value::Bool(*value),
        Value::Int(value) => serde_json::Value::Number((*value).into()),
        Value::Float(value) => serde_json::Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::String(text) | Value::Symbol(text) | Value::Keyword(text) => {
            serde_json::Value::String(text.clone())
        }
        Value::List(items) => {
            let is_object = items.iter().all(|item| {
                item.as_list().is_some_and(|pair| {
                    pair.len() == 2 && pair.first().and_then(Value::as_keyword).is_some()
                })
            });
            if is_object && !items.is_empty() {
                let mut object = serde_json::Map::new();
                for item in items {
                    let pair = item.as_list().unwrap();
                    object.insert(
                        pair[0].as_keyword().unwrap().to_string(),
                        value_to_json(&pair[1]),
                    );
                }
                serde_json::Value::Object(object)
            } else {
                serde_json::Value::Array(items.iter().map(value_to_json).collect())
            }
        }
    }
}

fn require<'a>(items: &'a [Value], index: usize, form: &str) -> Result<&'a Value> {
    items
        .get(index)
        .ok_or_else(|| anyhow!("{form} needs argument {index}"))
}

fn parse_defun(list: &[Value]) -> Result<(String, Function)> {
    let name = list
        .get(1)
        .and_then(Value::as_symbol)
        .ok_or_else(|| anyhow!("defun needs a name"))?
        .to_string();
    let params = list
        .get(2)
        .and_then(Value::as_list)
        .ok_or_else(|| anyhow!("defun needs a parameter list"))?
        .iter()
        .map(|value| {
            value
                .as_symbol()
                .map(str::to_string)
                .ok_or_else(|| anyhow!("defun parameter is not a symbol"))
        })
        .collect::<Result<Vec<_>>>()?;
    let body = list[3..].to_vec();
    if body.is_empty() {
        bail!("defun needs a body");
    }
    Ok((name, Function { params, body }))
}

fn truthy(value: &Value) -> bool {
    !matches!(value, Value::Nil | Value::Bool(false))
}

fn is_tag(value: &Value, tag: &str) -> bool {
    value
        .as_list()
        .and_then(|items| items.first())
        .and_then(Value::as_symbol)
        == Some(tag)
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(text) | Value::Symbol(text) | Value::Keyword(text) => text.clone(),
        other => other.to_string(),
    }
}

fn display_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn field_value(record: &Value, name: &Value) -> Value {
    let Some(items) = record.as_list() else {
        return Value::Nil;
    };
    let key = name.as_keyword().or_else(|| name.as_symbol());
    let Some(key) = key else {
        return Value::Nil;
    };
    for item in items {
        let Some(pair) = item.as_list() else { continue };
        let field = pair
            .first()
            .and_then(|value| value.as_keyword().or_else(|| value.as_symbol()));
        if field == Some(key) {
            return pair.get(1).cloned().unwrap_or(Value::Nil);
        }
    }
    Value::Nil
}

/// A deterministic in-memory records host for the first runtime slice.
pub struct FakeHost {
    records: HashMap<String, Vec<Value>>,
    allow_all: bool,
    grants: Vec<(String, String)>,
}

impl Default for FakeHost {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeHost {
    pub fn new() -> Self {
        let mut records = HashMap::new();
        records.insert(
            "signal:avery".to_string(),
            vec![
                record_value("signal:avery", "pine:1:hr", br#"{"bpm":61}"#),
                record_value("signal:avery", "pine:1:motion", br#"{"moving":true}"#),
            ],
        );
        records.insert(
            "device:avery".to_string(),
            vec![record_value(
                "device:avery",
                "pine:1",
                br#"{"kind":"wearable.watch"}"#,
            )],
        );

        Self {
            records,
            allow_all: true,
            grants: Vec::new(),
        }
    }

    pub fn deny_all(mut self) -> Self {
        self.allow_all = false;
        self.grants.clear();
        self
    }

    pub fn grant(mut self, interface: &str, function: &str) -> Self {
        self.allow_all = false;
        self.grants
            .push((interface.to_string(), function.to_string()));
        self
    }

    pub fn records(&self) -> &HashMap<String, Vec<Value>> {
        &self.records
    }
}

impl Host for FakeHost {
    fn granted(&self, interface: &str, function: &str) -> bool {
        self.allow_all
            || self
                .grants
                .iter()
                .any(|(i, f)| i == interface && f == function)
    }

    fn call(
        &mut self,
        interface: &str,
        function: &str,
        args: &[Value],
    ) -> std::result::Result<Value, String> {
        if interface != "soil:records" {
            return Err(format!("unknown interface `{interface}`"));
        }

        match function {
            "scan" | "changes" => {
                let collection = args
                    .first()
                    .map(value_text)
                    .ok_or_else(|| "scan needs a collection".to_string())?;
                let limit = args.get(2).and_then(Value::as_f64).unwrap_or(50.0) as usize;
                let entries = self
                    .records
                    .get(&collection)
                    .map(|records| records.iter().take(limit).cloned().collect())
                    .unwrap_or_default();
                Ok(ok(page_value(entries)))
            }
            "get" => {
                let key = args.first().ok_or_else(|| "get needs a key".to_string())?;
                let (collection, id) = parse_key(key)?;
                let found = self
                    .records
                    .get(&collection)
                    .and_then(|records| {
                        records.iter().find(|record| {
                            record
                                .as_list()
                                .map(|items| {
                                    field_value(
                                        &Value::List(items.to_vec()),
                                        &Value::Keyword("key".to_string()),
                                    )
                                    .as_list()
                                    .map(|key| {
                                        field_value(
                                            &Value::List(key.to_vec()),
                                            &Value::Keyword("id".to_string()),
                                        )
                                        .as_str()
                                            == Some(&id)
                                    })
                                    .unwrap_or(false)
                                })
                                .unwrap_or(false)
                        })
                    })
                    .cloned();
                Ok(match found {
                    Some(record) => ok(record),
                    None => err("not-found"),
                })
            }
            "upsert" => {
                let entry = args
                    .first()
                    .cloned()
                    .ok_or_else(|| "upsert needs an entry".to_string())?;
                let key = field_value(&entry, &Value::Keyword("key".to_string()));
                let (collection, id) = parse_key(&key)?;
                let mut records = self.records.remove(&collection).unwrap_or_default();
                records.retain(|record| {
                    let key = field_value(record, &Value::Keyword("key".to_string()));
                    parse_key(&key)
                        .map(|(_, current)| current != id)
                        .unwrap_or(true)
                });
                records.push(entry.clone());
                self.records.insert(collection, records);
                Ok(ok(entry))
            }
            "delete" => {
                let key = args
                    .first()
                    .ok_or_else(|| "delete needs a key".to_string())?;
                let (collection, id) = parse_key(key)?;
                if let Some(records) = self.records.get_mut(&collection) {
                    records.retain(|record| {
                        let key = field_value(record, &Value::Keyword("key".to_string()));
                        parse_key(&key)
                            .map(|(_, current)| current != id)
                            .unwrap_or(true)
                    });
                }
                Ok(ok(Value::Nil))
            }
            "scan-packed" | "cas" => Ok(err("unsupported")),
            other => Err(format!("unknown soil:records function `{other}`")),
        }
    }
}

fn parse_key(value: &Value) -> Result<(String, String), String> {
    let collection = field_value(value, &Value::Keyword("collection".to_string()))
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "key needs a collection".to_string())?;
    let id = field_value(value, &Value::Keyword("id".to_string()))
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "key needs an id".to_string())?;
    Ok((collection, id))
}

fn pair(name: &str, value: Value) -> Value {
    Value::List(vec![Value::Keyword(name.to_string()), value])
}

fn key_value(collection: &str, id: &str) -> Value {
    Value::List(vec![
        pair("collection", Value::String(collection.to_string())),
        pair("id", Value::String(id.to_string())),
    ])
}

fn record_value(collection: &str, id: &str, body: &[u8]) -> Value {
    Value::List(vec![
        pair("key", key_value(collection, id)),
        pair("schema", Value::String("signal.sample.v1".to_string())),
        pair("revision", Value::Int(1)),
        pair("updated-at-ms", Value::Int(0)),
        pair("origin", Value::String("fake".to_string())),
        pair("deleted", Value::Bool(false)),
        pair(
            "body",
            Value::List(body.iter().map(|byte| Value::Int(*byte as i64)).collect()),
        ),
    ])
}

fn page_value(entries: Vec<Value>) -> Value {
    Value::List(vec![
        pair("entries", Value::List(entries)),
        pair("next", Value::Nil),
    ])
}

fn ok(value: Value) -> Value {
    Value::List(vec![Value::Symbol("ok".to_string()), value])
}

fn err(code: &str) -> Value {
    Value::List(vec![
        Value::Symbol("err".to_string()),
        Value::Symbol(code.to_string()),
    ])
}
