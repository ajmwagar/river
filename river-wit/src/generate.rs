//! WIT -> S-expression bindings.
//!
//! The output is deliberately small:
//! - type definitions,
//! - capability metadata for introspection,
//! - one `defun` stub per function that calls `(host-call interface function args)`.
//!
//! There is no runtime catalog and no per-capability host adapter. The host
//! (the Soil web bridge at `/v1/call`, or a host-embedded `__soil.invoke`)
//! owns grants and dispatch.

use std::fmt;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::Result;
use wit_parser::{Resolve, Type, TypeDefKind};

pub struct Generated {
    pub package: String,
    pub sexp: String,
    pub capabilities: usize,
    pub types: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TypeRef {
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
struct Field {
    name: String,
    ty: TypeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VariantCase {
    name: String,
    ty: Option<TypeRef>,
}

enum TypeForm {
    Record(Vec<Field>),
    Variant(Vec<VariantCase>),
    Enum(Vec<String>),
    Flags(Vec<String>),
    Alias(TypeRef),
    Resource,
    Unsupported(String),
}

pub fn generate(path: impl AsRef<Path>) -> Result<Generated> {
    let mut resolve = Resolve::default();
    let (package_id, _sources) = resolve.push_path(path.as_ref())?;
    let package = &resolve.packages[package_id];

    let package_full = package.name.to_string();
    let base = format!("{}:{}", package.name.namespace, package.name.name);
    let version = package
        .name
        .version
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();

    let mut sexp = String::new();
    let mut capabilities = 0;
    let mut types = 0;

    let _ = writeln!(
        sexp,
        "(define-package {package_full:?} :base {base:?} :version {version:?})\n"
    );

    for (interface_key, interface_id) in &package.interfaces {
        let interface = &resolve.interfaces[*interface_id];
        let interface_name = interface
            .name
            .clone()
            .unwrap_or_else(|| interface_key.clone());

        for (type_name, type_id) in &interface.types {
            let type_def = &resolve.types[*type_id];
            let full_name = scoped_name(&base, &interface_name, type_name);
            let form = type_form(&resolve, &type_def.kind, &base, &interface_name);
            write_type(&mut sexp, &full_name, &form);
            types += 1;
        }

        for (function_name, function) in &interface.functions {
            let capability_name = scoped_name(&base, &interface_name, function_name);

            let params = function
                .params
                .iter()
                .enumerate()
                .map(|(index, param)| Field {
                    name: if param.name.is_empty() {
                        format!("arg{index}")
                    } else {
                        param.name.clone()
                    },
                    ty: type_ref(&resolve, param.ty, &base, &interface_name),
                })
                .collect::<Vec<_>>();

            let result = function
                .result
                .map(|ty| type_ref(&resolve, ty, &base, &interface_name));

            write_capability(
                &mut sexp,
                &capability_name,
                &base,
                function_name,
                &base,
                &version,
                &params,
                result.as_ref(),
            );
            write_stub(&mut sexp, &capability_name, &base, function_name, &params);
            capabilities += 1;
        }
    }

    Ok(Generated {
        package: package_full,
        sexp,
        capabilities,
        types,
    })
}

fn write_capability(
    sexp: &mut String,
    capability_name: &str,
    interface: &str,
    function: &str,
    grant: &str,
    version: &str,
    params: &[Field],
    result: Option<&TypeRef>,
) {
    let params_text = params
        .iter()
        .map(|param| format!("({} {})", param.name, param.ty))
        .collect::<Vec<_>>()
        .join(" ");
    let result_text = result
        .map(ToString::to_string)
        .unwrap_or_else(|| "unit".to_string());

    let _ = writeln!(sexp, "(define-capability {capability_name}");
    let _ = writeln!(sexp, "  :interface {interface:?}");
    let _ = writeln!(sexp, "  :function {function:?}");
    let _ = writeln!(sexp, "  :grant {grant:?}");
    let _ = writeln!(sexp, "  :version {version:?}");
    let _ = writeln!(sexp, "  :params ({params_text})");
    let _ = writeln!(sexp, "  :result {result_text})\n");
}

fn write_stub(
    sexp: &mut String,
    capability_name: &str,
    interface: &str,
    function: &str,
    params: &[Field],
) {
    let names = params
        .iter()
        .map(|param| param.name.clone())
        .collect::<Vec<_>>()
        .join(" ");
    let args = if names.is_empty() {
        "nil".to_string()
    } else {
        format!("(list {names})")
    };

    let _ = writeln!(sexp, "(defun {capability_name} ({names})");
    let _ = writeln!(sexp, "  (host-call {interface:?} {function:?} {args}))\n");
}

fn scoped_name(base: &str, interface: &str, name: &str) -> String {
    let package_name = base.split(':').nth(1).unwrap_or("");
    if interface == package_name {
        format!("{base}/{name}")
    } else {
        format!("{base}/{interface}.{name}")
    }
}

fn type_ref(resolve: &Resolve, ty: Type, base: &str, interface: &str) -> TypeRef {
    match ty {
        Type::Bool => TypeRef::Primitive("bool".to_string()),
        Type::U8 => TypeRef::Primitive("u8".to_string()),
        Type::U16 => TypeRef::Primitive("u16".to_string()),
        Type::U32 => TypeRef::Primitive("u32".to_string()),
        Type::U64 => TypeRef::Primitive("u64".to_string()),
        Type::S8 => TypeRef::Primitive("s8".to_string()),
        Type::S16 => TypeRef::Primitive("s16".to_string()),
        Type::S32 => TypeRef::Primitive("s32".to_string()),
        Type::S64 => TypeRef::Primitive("s64".to_string()),
        Type::F32 => TypeRef::Primitive("f32".to_string()),
        Type::F64 => TypeRef::Primitive("f64".to_string()),
        Type::Char => TypeRef::Primitive("char".to_string()),
        Type::String => TypeRef::Primitive("string".to_string()),
        Type::ErrorContext => TypeRef::Primitive("error-context".to_string()),
        Type::Id(id) => {
            let type_def = &resolve.types[id];
            match &type_def.name {
                Some(name) => TypeRef::Named(scoped_name(base, interface, name)),
                None => kind_to_ref(resolve, &type_def.kind, base, interface),
            }
        }
    }
}

fn kind_to_ref(resolve: &Resolve, kind: &TypeDefKind, base: &str, interface: &str) -> TypeRef {
    match kind {
        TypeDefKind::Record(record) => TypeRef::Record(
            record
                .fields
                .iter()
                .map(|field| Field {
                    name: field.name.clone(),
                    ty: type_ref(resolve, field.ty, base, interface),
                })
                .collect(),
        ),
        TypeDefKind::Variant(variant) => TypeRef::Variant(
            variant
                .cases
                .iter()
                .map(|case| VariantCase {
                    name: case.name.clone(),
                    ty: case.ty.map(|ty| type_ref(resolve, ty, base, interface)),
                })
                .collect(),
        ),
        TypeDefKind::Enum(enumeration) => TypeRef::Enum(
            enumeration
                .cases
                .iter()
                .map(|case| case.name.clone())
                .collect(),
        ),
        TypeDefKind::Flags(flags) => {
            TypeRef::Flags(flags.flags.iter().map(|flag| flag.name.clone()).collect())
        }
        TypeDefKind::Tuple(tuple) => TypeRef::Tuple(
            tuple
                .types
                .iter()
                .map(|ty| type_ref(resolve, *ty, base, interface))
                .collect(),
        ),
        TypeDefKind::Option(inner) => {
            TypeRef::Option(Box::new(type_ref(resolve, *inner, base, interface)))
        }
        TypeDefKind::Result(result) => TypeRef::Result {
            ok: Box::new(option_type_ref(resolve, result.ok, base, interface)),
            err: Box::new(option_type_ref(resolve, result.err, base, interface)),
        },
        TypeDefKind::List(inner) => {
            TypeRef::List(Box::new(type_ref(resolve, *inner, base, interface)))
        }
        TypeDefKind::Type(inner) => type_ref(resolve, *inner, base, interface),
        TypeDefKind::Resource => TypeRef::Unsupported("resource".to_string()),
        TypeDefKind::Handle(_) => TypeRef::Unsupported("handle".to_string()),
        TypeDefKind::Map(_, _) => TypeRef::Unsupported("map".to_string()),
        TypeDefKind::FixedLengthList(_, _) => TypeRef::Unsupported("fixed-length-list".to_string()),
        TypeDefKind::Future(_) => TypeRef::Unsupported("future".to_string()),
        TypeDefKind::Stream(_) => TypeRef::Unsupported("stream".to_string()),
        _ => TypeRef::Unsupported("unknown".to_string()),
    }
}

fn option_type_ref(resolve: &Resolve, ty: Option<Type>, base: &str, interface: &str) -> TypeRef {
    match ty {
        Some(ty) => type_ref(resolve, ty, base, interface),
        None => TypeRef::Primitive("unit".to_string()),
    }
}

fn type_form(resolve: &Resolve, kind: &TypeDefKind, base: &str, interface: &str) -> TypeForm {
    match kind {
        TypeDefKind::Record(record) => TypeForm::Record(
            record
                .fields
                .iter()
                .map(|field| Field {
                    name: field.name.clone(),
                    ty: type_ref(resolve, field.ty, base, interface),
                })
                .collect(),
        ),
        TypeDefKind::Variant(variant) => TypeForm::Variant(
            variant
                .cases
                .iter()
                .map(|case| VariantCase {
                    name: case.name.clone(),
                    ty: case.ty.map(|ty| type_ref(resolve, ty, base, interface)),
                })
                .collect(),
        ),
        TypeDefKind::Enum(enumeration) => TypeForm::Enum(
            enumeration
                .cases
                .iter()
                .map(|case| case.name.clone())
                .collect(),
        ),
        TypeDefKind::Flags(flags) => {
            TypeForm::Flags(flags.flags.iter().map(|flag| flag.name.clone()).collect())
        }
        TypeDefKind::Type(inner) => TypeForm::Alias(type_ref(resolve, *inner, base, interface)),
        TypeDefKind::Tuple(tuple) => TypeForm::Alias(TypeRef::Tuple(
            tuple
                .types
                .iter()
                .map(|ty| type_ref(resolve, *ty, base, interface))
                .collect(),
        )),
        TypeDefKind::Option(inner) => TypeForm::Alias(TypeRef::Option(Box::new(type_ref(
            resolve, *inner, base, interface,
        )))),
        TypeDefKind::Result(result) => TypeForm::Alias(TypeRef::Result {
            ok: Box::new(option_type_ref(resolve, result.ok, base, interface)),
            err: Box::new(option_type_ref(resolve, result.err, base, interface)),
        }),
        TypeDefKind::List(inner) => TypeForm::Alias(TypeRef::List(Box::new(type_ref(
            resolve, *inner, base, interface,
        )))),
        TypeDefKind::Resource => TypeForm::Resource,
        TypeDefKind::Handle(_) => TypeForm::Unsupported("handle".to_string()),
        TypeDefKind::Map(_, _) => TypeForm::Unsupported("map".to_string()),
        TypeDefKind::FixedLengthList(_, _) => {
            TypeForm::Unsupported("fixed-length-list".to_string())
        }
        TypeDefKind::Future(_) => TypeForm::Unsupported("future".to_string()),
        TypeDefKind::Stream(_) => TypeForm::Unsupported("stream".to_string()),
        _ => TypeForm::Unsupported("unknown".to_string()),
    }
}

fn write_type(sexp: &mut String, name: &str, form: &TypeForm) {
    match form {
        TypeForm::Record(fields) => {
            let _ = writeln!(sexp, "(define-record {name}");
            for field in fields {
                let _ = writeln!(sexp, "  ({} {})", field.name, field.ty);
            }
            let _ = writeln!(sexp, ")\n");
        }
        TypeForm::Variant(cases) => {
            let _ = writeln!(sexp, "(define-variant {name}");
            for case in cases {
                match &case.ty {
                    Some(ty) => {
                        let _ = writeln!(sexp, "  ({} {ty})", case.name);
                    }
                    None => {
                        let _ = writeln!(sexp, "  ({})", case.name);
                    }
                }
            }
            let _ = writeln!(sexp, ")\n");
        }
        TypeForm::Enum(cases) => {
            let _ = writeln!(sexp, "(define-enum {name} {})", cases.join(" "));
            let _ = writeln!(sexp);
        }
        TypeForm::Flags(flags) => {
            let _ = writeln!(sexp, "(define-flags {name} {})", flags.join(" "));
            let _ = writeln!(sexp);
        }
        TypeForm::Alias(ty) => {
            let _ = writeln!(sexp, "(define-alias {name} {ty})\n");
        }
        TypeForm::Resource => {
            let _ = writeln!(sexp, "(define-resource {name})\n");
        }
        TypeForm::Unsupported(reason) => {
            let _ = writeln!(sexp, "(define-unsupported {name} {reason:?})\n");
        }
    }
}
