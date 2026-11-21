//! Lisp-visible introspection and capability dispatch.
//!
//! These are the forms the agent can evaluate in the same S-expression world
//! as the profiles:
//!
//! ```lisp
//! (capabilities)
//! (grants)
//! (interfaces)
//! (types)
//! (describe "soil:records/scan")
//! (soil:records/scan "signal:avery" nil 50)
//! ```

use river_eval::Value;

use crate::catalog::{Capability, Catalog, Host, HostError};
use crate::host::call_form;

pub fn eval_form(catalog: &Catalog, host: &mut dyn Host, form: &Value) -> Result<Value, HostError> {
    let list = form
        .as_list()
        .ok_or_else(|| HostError("introspection form must be a list".to_string()))?;
    let head = list
        .first()
        .and_then(Value::as_symbol)
        .ok_or_else(|| HostError("introspection form needs a name".to_string()))?;

    match head {
        "capabilities" => Ok(string_list(
            catalog.capabilities().iter().map(|cap| cap.name.clone()),
        )),
        "grants" => Ok(string_list(catalog.grants())),
        "interfaces" => Ok(string_list(catalog.interfaces())),
        "types" => Ok(string_list(catalog.types.iter().map(|ty| ty.name.clone()))),
        "packages" => Ok(Value::List(vec![Value::String(catalog.package.clone())])),
        "describe" => {
            let name = list
                .get(1)
                .and_then(Value::as_str)
                .ok_or_else(|| HostError("describe needs a capability name".to_string()))?;
            let capability = catalog
                .describe(name)
                .ok_or_else(|| HostError(format!("unknown capability `{name}`")))?;
            Ok(describe_value(capability))
        }
        _ => call_form(catalog, host, form),
    }
}

fn string_list(values: impl IntoIterator<Item = String>) -> Value {
    Value::List(values.into_iter().map(Value::String).collect())
}

fn describe_value(capability: &Capability) -> Value {
    let params = capability
        .params
        .iter()
        .map(|param| {
            Value::List(vec![
                Value::String(param.name.clone()),
                Value::String(param.ty.to_string()),
            ])
        })
        .collect();

    Value::List(vec![
        Value::Keyword("capability".to_string()),
        Value::String(capability.name.clone()),
        Value::List(vec![
            Value::Keyword("signature".to_string()),
            Value::String(capability.signature()),
        ]),
        Value::List(vec![
            Value::Keyword("grant".to_string()),
            Value::String(capability.grant.clone()),
        ]),
        Value::List(vec![
            Value::Keyword("version".to_string()),
            Value::String(capability.version.clone()),
        ]),
        Value::List(vec![
            Value::Keyword("params".to_string()),
            Value::List(params),
        ]),
        Value::List(vec![
            Value::Keyword("result".to_string()),
            Value::String(
                capability
                    .result
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "unit".to_string()),
            ),
        ]),
    ])
}
