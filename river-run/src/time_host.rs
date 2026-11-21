//! Minimal in-process `soil:time` host.

use std::time::{SystemTime, UNIX_EPOCH};

use river_eval::sexp::Value;

use crate::Host;

pub struct SystemTimeHost;

impl SystemTimeHost {
    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(0)
    }
}

impl Host for SystemTimeHost {
    fn granted(&self, interface: &str, function: &str) -> bool {
        interface == "soil:time" && matches!(function, "monotonic-ms" | "wall-clock" | "tick")
    }

    fn call(
        &mut self,
        interface: &str,
        function: &str,
        _args: &[Value],
    ) -> std::result::Result<Value, String> {
        if interface != "soil:time" {
            return Err(format!("unknown interface `{interface}`"));
        }
        let now = Self::now_ms();
        match function {
            "monotonic-ms" => Ok(ok(Value::Int(now))),
            "wall-clock" => Ok(ok(Value::List(vec![
                pair("unix-ms", Value::Int(now)),
                pair("offset-minutes", Value::Int(0)),
            ]))),
            "tick" => Ok(ok(Value::Nil)),
            other => Err(format!("unknown soil:time function `{other}`")),
        }
    }
}

fn pair(name: &str, value: Value) -> Value {
    Value::List(vec![Value::Keyword(name.to_string()), value])
}

fn ok(value: Value) -> Value {
    Value::List(vec![Value::Symbol("ok".to_string()), value])
}
