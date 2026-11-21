//! Calling generated capabilities from Lisp-shaped forms.

use river_eval::Value;

use crate::catalog::{Catalog, Host, HostError};

/// Evaluate a generated capability call:
///
/// ```lisp
/// (soil:records/scan "signal:avery" nil 50)
/// ```
///
/// The first symbol is the capability name; the remaining values are its
/// arguments. Grant, arity, and basic type checks happen in `Catalog::call`.
pub fn call_form(catalog: &Catalog, host: &mut dyn Host, form: &Value) -> Result<Value, HostError> {
    let list = form
        .as_list()
        .ok_or_else(|| HostError("capability call must be a list".to_string()))?;
    let name = list
        .first()
        .and_then(Value::as_symbol)
        .ok_or_else(|| HostError("capability call needs a name".to_string()))?;
    catalog.call(host, name, &list[1..])
}
