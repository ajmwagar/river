//! The first River evaluator: wearable presence profiles.
//!
//! This is a closed evaluator, not a general Lisp. It understands `profile`,
//! `fusion`, `cond`, `state`, and the rule calls the presence profiles use.
//! That is enough to port Instinct's semantics before designing a compiler.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use crate::sexp::{self, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Connected,
    Offline,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Good,
    Degraded,
    Unknown,
    Denied,
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub kind: String,
    pub device_id: String,
    pub wearer_id: Option<String>,
    pub observed_at_ms: i64,
    pub quality: Quality,
    pub value: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub id: String,
    pub kind: String,
    pub wearer_id: Option<String>,
    pub bridge_id: Option<String>,
    pub link: Link,
    pub charging: bool,
    pub battery: Option<i64>,
    pub last_seen_ms: i64,
    pub signals: Vec<Signal>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WearableState {
    pub device_id: String,
    pub wearer_id: Option<String>,
    pub state: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub since_ms: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersonState {
    pub presence: String,
    pub confidence: f64,
    pub evidence: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub kinds: Vec<String>,
    pub classify: Value,
}

#[derive(Debug, Clone)]
pub struct Fusion {
    pub weights: HashMap<String, f64>,
    pub otherwise: String,
}

#[derive(Debug, Clone)]
pub struct Program {
    profiles: Vec<Profile>,
    by_kind: HashMap<String, usize>,
    fusion: Fusion,
}

#[derive(Debug, Clone)]
pub struct EvalError(pub String);

impl fmt::Display for EvalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for EvalError {}

fn error(message: impl Into<String>) -> EvalError {
    EvalError(message.into())
}

pub fn load_program(directory: impl AsRef<Path>) -> Result<Program, EvalError> {
    let directory = directory.as_ref();
    let mut files = Vec::new();

    for entry in std::fs::read_dir(directory)
        .map_err(|err| error(format!("read {}: {err}", directory.display())))?
    {
        let entry = entry.map_err(|err| error(format!("read directory entry: {err}")))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("river") {
            files.push(path);
        }
    }
    files.sort();

    let mut profiles = Vec::new();
    let mut fusion = None;

    for file in files {
        let source = std::fs::read_to_string(&file)
            .map_err(|err| error(format!("read {}: {err}", file.display())))?;
        let forms =
            sexp::parse_all(&source).map_err(|err| error(format!("{}: {err}", file.display())))?;

        for form in forms {
            let list = form.as_list().ok_or_else(|| {
                error(format!("{}: top-level form is not a list", file.display()))
            })?;
            let head = list
                .first()
                .and_then(Value::as_symbol)
                .ok_or_else(|| error(format!("{}: top-level form has no name", file.display())))?;

            match head {
                "profile" => profiles.push(parse_profile(list)?),
                "fusion" => fusion = Some(parse_fusion(list)?),
                other => {
                    return Err(error(format!(
                        "{}: unknown top-level form `{other}`",
                        file.display()
                    )))
                }
            }
        }
    }

    let mut by_kind = HashMap::new();
    for (index, profile) in profiles.iter().enumerate() {
        for kind in &profile.kinds {
            if by_kind.insert(kind.clone(), index).is_some() {
                return Err(error(format!("duplicate profile for kind `{kind}`")));
            }
        }
    }

    Ok(Program {
        profiles,
        by_kind,
        fusion: fusion.unwrap_or(Fusion {
            weights: HashMap::new(),
            otherwise: "unknown".to_string(),
        }),
    })
}

fn parse_profile(form: &[Value]) -> Result<Profile, EvalError> {
    let name = form
        .get(1)
        .and_then(Value::as_symbol)
        .ok_or_else(|| error("profile needs a name"))?
        .to_string();

    let mut kinds = Vec::new();
    let mut classify = None;

    for item in &form[2..] {
        let list = item
            .as_list()
            .ok_or_else(|| error(format!("profile `{name}`: expected a list")))?;
        let head = list
            .first()
            .and_then(Value::as_symbol)
            .ok_or_else(|| error(format!("profile `{name}`: empty clause")))?;

        match head {
            "kind" => {
                for value in &list[1..] {
                    kinds.push(keyword_to_string(value)?);
                }
            }
            "signals" => {}
            "classify" => {
                classify = Some(
                    list.get(1)
                        .ok_or_else(|| error(format!("profile `{name}`: classify needs a body")))?
                        .clone(),
                );
            }
            other => return Err(error(format!("profile `{name}`: unknown clause `{other}`"))),
        }
    }

    Ok(Profile {
        name,
        kinds,
        classify: classify.ok_or_else(|| error("profile needs a classify clause"))?,
    })
}

fn parse_fusion(form: &[Value]) -> Result<Fusion, EvalError> {
    let mut weights = HashMap::new();
    let mut otherwise = "unknown".to_string();

    for item in &form[2..] {
        let list = item
            .as_list()
            .ok_or_else(|| error("fusion: expected a list"))?;
        let head = list
            .first()
            .and_then(Value::as_symbol)
            .ok_or_else(|| error("fusion: empty clause"))?;

        match head {
            "weigh" => {
                let state = keyword_to_string(
                    list.get(1)
                        .ok_or_else(|| error("fusion weigh needs a state"))?,
                )?;
                let weight = list
                    .get(2)
                    .and_then(Value::as_f64)
                    .ok_or_else(|| error("fusion weigh needs a number"))?;
                weights.insert(state, weight);
            }
            "otherwise" => {
                otherwise = keyword_to_string(
                    list.get(1)
                        .ok_or_else(|| error("fusion otherwise needs a state"))?,
                )?;
            }
            other => return Err(error(format!("fusion: unknown clause `{other}`"))),
        }
    }

    Ok(Fusion { weights, otherwise })
}

fn keyword_to_string(value: &Value) -> Result<String, EvalError> {
    value
        .as_keyword()
        .map(str::to_string)
        .ok_or_else(|| error(format!("expected a keyword, got `{value}`")))
}

impl Program {
    pub fn classify(&self, device: &Device, now_ms: i64) -> Result<WearableState, EvalError> {
        let index = *self.by_kind.get(&device.kind).ok_or_else(|| {
            error(format!(
                "no profile for device kind `{}` (device `{}`)",
                device.kind, device.id
            ))
        })?;
        let profile = &self.profiles[index];
        let context = Context { device, now_ms };

        match evaluate(&profile.classify, &context)? {
            Eval::State {
                state,
                confidence,
                evidence,
            } => Ok(WearableState {
                device_id: device.id.clone(),
                wearer_id: device.wearer_id.clone(),
                state,
                confidence,
                evidence: evidence.into_iter().collect(),
                since_ms: device.last_seen_ms,
            }),
            other => Err(error(format!(
                "profile `{}` returned {other:?}, not a state",
                profile.name
            ))),
        }
    }

    pub fn classify_all(
        &self,
        devices: &[Device],
        now_ms: i64,
    ) -> Result<Vec<WearableState>, EvalError> {
        devices
            .iter()
            .map(|device| self.classify(device, now_ms))
            .collect()
    }

    pub fn person(&self, states: &[WearableState]) -> PersonState {
        let mut best: Option<(f64, &WearableState)> = None;

        for state in states {
            let weight = self
                .fusion
                .weights
                .get(&state.state)
                .copied()
                .unwrap_or(0.0);
            if weight <= 0.0 {
                continue;
            }
            let score = weight * state.confidence;
            if best
                .as_ref()
                .is_none_or(|(best_score, _)| score > *best_score)
            {
                best = Some((score, state));
            }
        }

        let evidence = states
            .iter()
            .map(|state| (state.device_id.clone(), state.state.clone()))
            .collect();

        match best {
            Some((score, _)) => PersonState {
                presence: "present".to_string(),
                confidence: round2(score),
                evidence,
            },
            None => PersonState {
                presence: self.fusion.otherwise.clone(),
                confidence: 0.0,
                evidence,
            },
        }
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[derive(Debug, Clone, Copy)]
struct Context<'a> {
    device: &'a Device,
    now_ms: i64,
}

#[derive(Debug, Clone, PartialEq)]
enum Eval {
    Bool(bool),
    Number(f64),
    State {
        state: String,
        confidence: f64,
        evidence: Option<String>,
    },
}

fn evaluate(expression: &Value, context: &Context<'_>) -> Result<Eval, EvalError> {
    match expression {
        Value::Bool(value) => Ok(Eval::Bool(*value)),
        Value::Int(value) => Ok(Eval::Number(*value as f64)),
        Value::Float(value) => Ok(Eval::Number(*value)),
        Value::List(items) => {
            let head = items
                .first()
                .and_then(Value::as_symbol)
                .ok_or_else(|| error(format!("empty or invalid expression `{expression}`")))?;

            match head {
                "cond" => evaluate_cond(&items[1..], context),
                "state" => {
                    let state = keyword_to_string(
                        items.get(1).ok_or_else(|| error("state needs a name"))?,
                    )?;
                    let confidence = items
                        .get(2)
                        .and_then(Value::as_f64)
                        .ok_or_else(|| error("state needs a confidence"))?;
                    Ok(Eval::State {
                        state,
                        confidence,
                        evidence: None,
                    })
                }
                "charging" => Ok(Eval::Bool(context.device.charging)),
                "linked" => Ok(Eval::Bool(context.device.link == Link::Connected)),
                "not" => {
                    let value = evaluate_bool(
                        items.get(1).ok_or_else(|| error("not needs an argument"))?,
                        context,
                    )?;
                    Ok(Eval::Bool(!value))
                }
                "and" => {
                    for value in &items[1..] {
                        if !evaluate_bool(value, context)? {
                            return Ok(Eval::Bool(false));
                        }
                    }
                    Ok(Eval::Bool(true))
                }
                "or" => {
                    for value in &items[1..] {
                        if evaluate_bool(value, context)? {
                            return Ok(Eval::Bool(true));
                        }
                    }
                    Ok(Eval::Bool(false))
                }
                "stale" => Ok(Eval::Bool(stale(
                    context.device,
                    context.now_ms,
                    number_argument(items, 1, "stale")?,
                ))),
                "hr-contact" => Ok(Eval::Bool(hr_contact(
                    context.device,
                    context.now_ms,
                    number_argument(items, 1, "hr-contact")?,
                ))),
                "motion" => Ok(Eval::Bool(motion(
                    context.device,
                    context.now_ms,
                    number_argument(items, 1, "motion")?,
                ))),
                "audio-active" => Ok(Eval::Bool(audio_active(
                    context.device,
                    context.now_ms,
                    number_argument(items, 1, "audio-active")?,
                ))),
                "button-recent" => Ok(Eval::Bool(button_recent(
                    context.device,
                    context.now_ms,
                    number_argument(items, 1, "button-recent")?,
                ))),
                "=" => {
                    let left = number_argument(items, 1, "=")?;
                    let right = number_argument(items, 2, "=")?;
                    Ok(Eval::Bool(left == right))
                }
                ">" => {
                    let left = number_argument(items, 1, ">")?;
                    let right = number_argument(items, 2, ">")?;
                    Ok(Eval::Bool(left > right))
                }
                "<" => {
                    let left = number_argument(items, 1, "<")?;
                    let right = number_argument(items, 2, "<")?;
                    Ok(Eval::Bool(left < right))
                }
                other => Err(error(format!("unknown form `{other}`"))),
            }
        }
        Value::Symbol(name) if name == "else" => Ok(Eval::Bool(true)),
        other => Err(error(format!("cannot evaluate `{other}`"))),
    }
}

fn evaluate_cond(clauses: &[Value], context: &Context<'_>) -> Result<Eval, EvalError> {
    for clause in clauses {
        let list = clause
            .as_list()
            .ok_or_else(|| error("cond clause is not a list"))?;
        let test = list
            .first()
            .ok_or_else(|| error("cond clause has no test"))?;
        let body = list
            .get(1)
            .ok_or_else(|| error("cond clause has no body"))?;

        let matched = match test {
            Value::Symbol(name) if name == "else" => true,
            _ => evaluate_bool(test, context)?,
        };

        if matched {
            let result = evaluate(body, context)?;
            let result = match result {
                Eval::State {
                    state,
                    confidence,
                    evidence,
                } => Eval::State {
                    state,
                    confidence,
                    evidence: evidence.or_else(|| Some(test.to_string())),
                },
                other => other,
            };
            return Ok(result);
        }
    }

    Err(error("cond had no matching clause"))
}

fn evaluate_bool(expression: &Value, context: &Context<'_>) -> Result<bool, EvalError> {
    match evaluate(expression, context)? {
        Eval::Bool(value) => Ok(value),
        other => Err(error(format!(
            "expected a boolean from `{expression}`, got {other:?}"
        ))),
    }
}

fn number_argument(items: &[Value], index: usize, form: &str) -> Result<f64, EvalError> {
    items
        .get(index)
        .and_then(Value::as_f64)
        .ok_or_else(|| error(format!("`{form}` needs a number at position {index}")))
}

fn stale(device: &Device, now_ms: i64, max_age_ms: f64) -> bool {
    let age = now_ms - device.last_seen_ms;
    age < 0 || age as f64 > max_age_ms
}

fn signal_good(signal: &Signal) -> bool {
    matches!(signal.quality, Quality::Good | Quality::Degraded)
}

fn signal_fresh(signal: &Signal, now_ms: i64, max_age_ms: f64) -> bool {
    let age = now_ms - signal.observed_at_ms;
    age >= 0 && age as f64 <= max_age_ms
}

fn recent<F>(device: &Device, kind: &str, now_ms: i64, max_age_ms: f64, predicate: F) -> bool
where
    F: Fn(&Signal) -> bool,
{
    device.signals.iter().any(|signal| {
        signal.kind == kind
            && signal_good(signal)
            && signal_fresh(signal, now_ms, max_age_ms)
            && predicate(signal)
    })
}

fn hr_contact(device: &Device, now_ms: i64, max_age_ms: f64) -> bool {
    recent(device, "hr", now_ms, max_age_ms, |signal| {
        signal.value.get("contact").and_then(Value::as_str) == Some("good")
    }) || recent(device, "hr_contact", now_ms, max_age_ms, |_| true)
}

fn motion(device: &Device, now_ms: i64, max_age_ms: f64) -> bool {
    recent(device, "motion", now_ms, max_age_ms, |signal| {
        signal
            .value
            .get("moving")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    })
}

fn audio_active(device: &Device, now_ms: i64, max_age_ms: f64) -> bool {
    recent(device, "audio", now_ms, max_age_ms, |signal| {
        signal
            .value
            .get("active")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}

fn button_recent(device: &Device, now_ms: i64, max_age_ms: f64) -> bool {
    recent(device, "button", now_ms, max_age_ms, |_| true)
}
