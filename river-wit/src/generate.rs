//! WIT -> S-expression bindings.
//!
//! This is deliberately not `wit-bindgen`: there is no guest codegen and no
//! canonical ABI here. It parses WIT, emits readable S-expressions, and builds
//! the catalog the host bridge and introspection use.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::Result;
use wit_parser::{Resolve, Type, TypeDefKind};

use crate::catalog::{Capability, Catalog, Field, TypeDefinition, TypeKind, TypeRef, VariantCase};

pub struct Generated {
    pub catalog: Catalog,
    pub sexp: String,
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

    let mut catalog = Catalog::new(package_full.clone(), version.clone());
    let mut sexp = String::new();

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
            let kind = type_kind(&resolve, &type_def.kind, &base, &interface_name);
            write_type(&mut sexp, &full_name, &kind);
            catalog.push_type(TypeDefinition {
                name: full_name,
                kind,
            });
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

            let params_text = params
                .iter()
                .map(|param| format!("({} {})", param.name, param.ty))
                .collect::<Vec<_>>()
                .join(" ");
            let result_text = result
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "unit".to_string());

            let _ = writeln!(sexp, "(define-function {capability_name}");
            let _ = writeln!(sexp, "  ({params_text})");
            let _ = writeln!(sexp, "  -> {result_text}");
            let _ = writeln!(sexp, "  :grant {base:?} :version {version:?})\n");

            catalog.push_capability(Capability {
                name: capability_name,
                package: package_full.clone(),
                interface: interface_name.clone(),
                function: function_name.clone(),
                grant: base.clone(),
                version: version.clone(),
                params,
                result,
            });
        }
    }

    Ok(Generated { catalog, sexp })
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

fn type_kind(resolve: &Resolve, kind: &TypeDefKind, base: &str, interface: &str) -> TypeKind {
    match kind {
        TypeDefKind::Record(record) => TypeKind::Record(
            record
                .fields
                .iter()
                .map(|field| Field {
                    name: field.name.clone(),
                    ty: type_ref(resolve, field.ty, base, interface),
                })
                .collect(),
        ),
        TypeDefKind::Variant(variant) => TypeKind::Variant(
            variant
                .cases
                .iter()
                .map(|case| VariantCase {
                    name: case.name.clone(),
                    ty: case.ty.map(|ty| type_ref(resolve, ty, base, interface)),
                })
                .collect(),
        ),
        TypeDefKind::Enum(enumeration) => TypeKind::Enum(
            enumeration
                .cases
                .iter()
                .map(|case| case.name.clone())
                .collect(),
        ),
        TypeDefKind::Flags(flags) => {
            TypeKind::Flags(flags.flags.iter().map(|flag| flag.name.clone()).collect())
        }
        TypeDefKind::Type(inner) => TypeKind::Alias(type_ref(resolve, *inner, base, interface)),
        TypeDefKind::Tuple(tuple) => TypeKind::Alias(TypeRef::Tuple(
            tuple
                .types
                .iter()
                .map(|ty| type_ref(resolve, *ty, base, interface))
                .collect(),
        )),
        TypeDefKind::Option(inner) => TypeKind::Alias(TypeRef::Option(Box::new(type_ref(
            resolve, *inner, base, interface,
        )))),
        TypeDefKind::Result(result) => TypeKind::Alias(TypeRef::Result {
            ok: Box::new(option_type_ref(resolve, result.ok, base, interface)),
            err: Box::new(option_type_ref(resolve, result.err, base, interface)),
        }),
        TypeDefKind::List(inner) => TypeKind::Alias(TypeRef::List(Box::new(type_ref(
            resolve, *inner, base, interface,
        )))),
        TypeDefKind::Resource => TypeKind::Resource,
        TypeDefKind::Handle(_) => TypeKind::Unsupported("handle".to_string()),
        TypeDefKind::Map(_, _) => TypeKind::Unsupported("map".to_string()),
        TypeDefKind::FixedLengthList(_, _) => {
            TypeKind::Unsupported("fixed-length-list".to_string())
        }
        TypeDefKind::Future(_) => TypeKind::Unsupported("future".to_string()),
        TypeDefKind::Stream(_) => TypeKind::Unsupported("stream".to_string()),
        _ => TypeKind::Unsupported("unknown".to_string()),
    }
}

fn write_type(sexp: &mut String, name: &str, kind: &TypeKind) {
    match kind {
        TypeKind::Record(fields) => {
            let _ = writeln!(sexp, "(define-record {name}");
            for field in fields {
                let _ = writeln!(sexp, "  ({} {})", field.name, field.ty);
            }
            let _ = writeln!(sexp, ")\n");
        }
        TypeKind::Variant(cases) => {
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
        TypeKind::Enum(cases) => {
            let _ = writeln!(sexp, "(define-enum {name} {})", cases.join(" "));
            let _ = writeln!(sexp);
        }
        TypeKind::Flags(flags) => {
            let _ = writeln!(sexp, "(define-flags {name} {})", flags.join(" "));
            let _ = writeln!(sexp);
        }
        TypeKind::Alias(ty) => {
            let _ = writeln!(sexp, "(define-alias {name} {ty})\n");
        }
        TypeKind::Resource => {
            let _ = writeln!(sexp, "(define-resource {name})\n");
        }
        TypeKind::Unsupported(reason) => {
            let _ = writeln!(sexp, "(define-unsupported {name} {reason:?})\n");
        }
    }
}
