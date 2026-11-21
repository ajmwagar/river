use std::path::Path;

use river_eval::Value;
use river_wit::catalog::{Host, HostError};
use river_wit::{eval_form, generate};

fn generated() -> river_wit::Generated {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../soil/wit/soil-records.wit");
    generate(path).expect("generate soil:records")
}

struct HostThatDoesNotMatter;

impl Host for HostThatDoesNotMatter {
    fn granted(&self, _capability: &str) -> bool {
        true
    }

    fn call(&mut self, _capability: &str, _args: &[Value]) -> Result<Value, HostError> {
        Ok(Value::Nil)
    }
}

fn eval(generated: &river_wit::Generated, source: &str) -> Value {
    let form = river_eval::sexp::parse_all(source)
        .expect("parse")
        .remove(0);
    eval_form(&generated.catalog, &mut HostThatDoesNotMatter, &form).expect("eval")
}

#[test]
fn lists_capabilities_grants_interfaces_and_types() {
    let generated = generated();

    let capabilities = eval(&generated, "(capabilities)");
    assert!(capabilities
        .as_list()
        .unwrap()
        .iter()
        .any(|value| value.as_str() == Some("soil:records/scan")));

    let grants = eval(&generated, "(grants)");
    assert!(grants
        .as_list()
        .unwrap()
        .iter()
        .any(|value| value.as_str() == Some("soil:records")));

    let interfaces = eval(&generated, "(interfaces)");
    assert!(interfaces
        .as_list()
        .unwrap()
        .iter()
        .any(|value| value.as_str() == Some("records")));

    let types = eval(&generated, "(types)");
    assert!(types
        .as_list()
        .unwrap()
        .iter()
        .any(|value| value.as_str() == Some("soil:records/entry")));
}

#[test]
fn describes_a_capability_as_data() {
    let generated = generated();
    let description = eval(&generated, r#"(describe "soil:records/scan")"#);
    let list = description.as_list().expect("description list");

    assert_eq!(list[0].as_keyword(), Some("capability"));
    assert_eq!(list[1].as_str(), Some("soil:records/scan"));

    let rendered = format!("{description}");
    assert!(rendered.contains(":signature"));
    assert!(rendered.contains("collection"));
    assert!(rendered.contains(":grant"));
    assert!(rendered.contains("soil:records"));
}

#[test]
fn unknown_capabilities_fail_loudly() {
    let generated = generated();
    let form = river_eval::sexp::parse_all(r#"(describe "soil:records/nope")"#)
        .expect("parse")
        .remove(0);
    let error =
        eval_form(&generated.catalog, &mut HostThatDoesNotMatter, &form).expect_err("unknown");
    assert!(error.to_string().contains("unknown capability"));
}
