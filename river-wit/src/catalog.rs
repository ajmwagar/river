//! The generated capability catalog, its host bridge, and introspection.

use std::collections::BTreeSet;
use std::fmt;

use river_eval::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    Primitive(String),
    Named(String),
    List(Box<TypeRef>),
    Option(Box<TypeRef>),
    Result { ok: Box<TypeRef>, err: Box<TypeRef> },
    Tuple(Vec<TypeRef>),
    Record(Vec<Field>),
    Variant(Vec<VariantCase>),
    Enum(Vec<String>),
    Flags(Vec<String>),
    Unsupported(String),
}

impl fmt::Display for TypeRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primitive(name) | Self::Named(name) => write!(formatter, "{name}"),
            Self::List(inner) => write!(formatter, "(list {inner})"),
            Self::Option(inner) => write!(formatter, "(option {inner})"),
            Self::Result { ok, err } => write!(formatter, "(result {ok} {err})"),
            Self::Tuple(items) => {
                write!(formatter, "(tuple")?;
                for item in items {
                    write!(formatter, " {item}")?;
                }
                write!(formatter, ")")
            }
            Self::Record(fields) => {
                write!(formatter, "(record")?;
                for field in fields {
                    write!(formatter, " ({} {})", field.name, field.ty)?;
                }
                write!(formatter, ")")
            }
            Self::Variant(cases) => {
                write!(formatter, "(variant")?;
                for case in cases {
                    match &case.ty {
                        Some(ty) => write!(formatter, " ({} {ty})", case.name)?,
                        None => write!(formatter, " ({})", case.name)?,
                    }
                }
                write!(formatter, ")")
            }
            Self::Enum(cases) => {
                write!(formatter, "(enum")?;
                for case in cases {
                    write!(formatter, " {case}")?;
                }
                write!(formatter, ")")
            }
            Self::Flags(flags) => {
                write!(formatter, "(flags")?;
                for flag in flags {
                    write!(formatter, " {flag}")?;
                }
                write!(formatter, ")")
            }
            Self::Unsupported(reason) => write!(formatter, "(unsupported {reason:?})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: TypeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantCase {
    pub name: String,
    pub ty: Option<TypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    Record(Vec<Field>),
    Variant(Vec<VariantCase>),
    Enum(Vec<String>),
    Flags(Vec<String>),
    Alias(TypeRef),
    Resource,
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDefinition {
    pub name: String,
    pub kind: TypeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub name: String,
    pub package: String,
    pub interface: String,
    pub function: String,
    pub grant: String,
    pub version: String,
    pub params: Vec<Field>,
    pub result: Option<TypeRef>,
}

impl Capability {
    pub fn signature(&self) -> String {
        let params = self
            .params
            .iter()
            .map(|param| format!("({} {})", param.name, param.ty))
            .collect::<Vec<_>>()
            .join(" ");
        let result = self
            .result
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "unit".to_string());
        format!("({} {params}) -> {result}", self.name)
    }
}

#[derive(Debug, Clone)]
pub struct Catalog {
    pub package: String,
    pub version: String,
    pub capabilities: Vec<Capability>,
    pub types: Vec<TypeDefinition>,
}

impl Catalog {
    pub fn new(package: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            package: package.into(),
            version: version.into(),
            capabilities: Vec::new(),
            types: Vec::new(),
        }
    }

    pub fn push_capability(&mut self, capability: Capability) {
        self.capabilities.push(capability);
    }

    pub fn push_type(&mut self, definition: TypeDefinition) {
        self.types.push(definition);
    }

    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    pub fn describe(&self, name: &str) -> Option<&Capability> {
        self.capabilities.iter().find(|cap| cap.name == name)
    }

    pub fn type_by_name(&self, name: &str) -> Option<&TypeDefinition> {
        self.types.iter().find(|ty| ty.name == name)
    }

    pub fn grants(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .map(|cap| cap.grant.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn interfaces(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .map(|cap| cap.interface.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn signature(&self, name: &str) -> Option<String> {
        self.describe(name).map(Capability::signature)
    }

    /// Call a generated capability through a host, after grant, arity, and
    /// basic value checks. The host owns the actual side effect.
    pub fn call(
        &self,
        host: &mut dyn Host,
        name: &str,
        args: &[Value],
    ) -> Result<Value, HostError> {
        let capability = self
            .describe(name)
            .ok_or_else(|| HostError(format!("unknown capability `{name}`")))?;

        if !host.granted(name) {
            return Err(HostError(format!(
                "denied: `{name}` is not granted to this caller"
            )));
        }

        if args.len() != capability.params.len() {
            return Err(HostError(format!(
                "`{name}` expects {} arguments, got {}",
                capability.params.len(),
                args.len()
            )));
        }

        for (index, (param, value)) in capability.params.iter().zip(args).enumerate() {
            validate_arg(&param.ty, value).map_err(|reason| {
                HostError(format!(
                    "`{name}` argument {index} (`{}`) {reason}",
                    param.name
                ))
            })?;
        }

        host.call(name, args)
    }
}

pub trait Host {
    fn granted(&self, capability: &str) -> bool;
    fn call(&mut self, capability: &str, args: &[Value]) -> Result<Value, HostError>;
}

#[derive(Debug, Clone)]
pub struct HostError(pub String);

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for HostError {}

fn validate_arg(ty: &TypeRef, value: &Value) -> Result<(), String> {
    match ty {
        TypeRef::Primitive(name) => match name.as_str() {
            "bool" => matches!(value, Value::Bool(_))
                .then_some(())
                .ok_or_else(|| "expects a boolean".to_string()),
            "string" | "char" | "error-context" => {
                matches!(value, Value::String(_) | Value::Symbol(_))
                    .then_some(())
                    .ok_or_else(|| "expects a string".to_string())
            }
            "u8" | "u16" | "u32" | "u64" | "s8" | "s16" | "s32" | "s64" => {
                matches!(value, Value::Int(_))
                    .then_some(())
                    .ok_or_else(|| "expects an integer".to_string())
            }
            "f32" | "f64" => matches!(value, Value::Int(_) | Value::Float(_))
                .then_some(())
                .ok_or_else(|| "expects a number".to_string()),
            "unit" => matches!(value, Value::Nil)
                .then_some(())
                .ok_or_else(|| "expects nil".to_string()),
            _ => Ok(()),
        },
        TypeRef::List(inner) => match value {
            Value::List(items) => {
                for item in items {
                    validate_arg(inner, item)?;
                }
                Ok(())
            }
            _ => Err("expects a list".to_string()),
        },
        TypeRef::Option(inner) => match value {
            Value::Nil => Ok(()),
            other => validate_arg(inner, other),
        },
        TypeRef::Result { ok, err } => match value {
            Value::List(items) if items.len() == 2 => match items[0].as_symbol() {
                Some("ok") => validate_arg(ok, &items[1]),
                Some("err") => validate_arg(err, &items[1]),
                _ => Err("expects `(ok value)` or `(err value)`".to_string()),
            },
            _ => Err("expects `(ok value)` or `(err value)`".to_string()),
        },
        TypeRef::Tuple(items) => match value {
            Value::List(values) if values.len() == items.len() => {
                for (ty, value) in items.iter().zip(values) {
                    validate_arg(ty, value)?;
                }
                Ok(())
            }
            _ => Err(format!("expects a {}-element tuple", items.len())),
        },
        TypeRef::Named(_)
        | TypeRef::Record(_)
        | TypeRef::Variant(_)
        | TypeRef::Enum(_)
        | TypeRef::Flags(_)
        | TypeRef::Unsupported(_) => Ok(()),
    }
}
