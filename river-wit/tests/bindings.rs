use std::path::Path;

use river_eval::Value;
use river_wit::catalog::{Host, HostError};
use river_wit::{call_form, generate};

fn generated() -> river_wit::Generated {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../soil/wit/soil-records.wit");
    generate(path).expect("generate soil:records")
}

#[test]
fn generates_records_and_capabilities() {
    let generated = generated();
    let text = &generated.sexp;

    assert!(text.contains("(define-package \"soil:records@0.1.0\""));
    assert!(text.contains("(define-record soil:records/entry"));
    assert!(text.contains("(define-enum soil:records/error"));
    assert!(text.contains("(define-function soil:records/scan"));

    let scan = generated
        .catalog
        .describe("soil:records/scan")
        .expect("scan capability");
    assert_eq!(scan.grant, "soil:records");
    assert_eq!(scan.params.len(), 3);
    assert_eq!(scan.params[0].name, "collection");
    assert_eq!(scan.params[0].ty.to_string(), "string");
    assert_eq!(scan.params[1].ty.to_string(), "(option string)");
    assert_eq!(scan.params[2].ty.to_string(), "u32");
    assert!(generated
        .catalog
        .grants()
        .contains(&"soil:records".to_string()));
}

#[test]
fn describes_types_and_signatures() {
    let generated = generated();
    let catalog = &generated.catalog;

    let signature = catalog
        .signature("soil:records/scan")
        .expect("scan signature");
    assert!(signature.contains("collection"));
    assert!(signature.contains("(result soil:records/page soil:records/error)"));

    let entry = catalog
        .type_by_name("soil:records/entry")
        .expect("entry type");
    assert!(matches!(
        &entry.kind,
        river_wit::TypeKind::Record(fields) if fields.iter().any(|field| field.name == "body")
    ));
}

struct TestHost {
    grant: bool,
}

impl Host for TestHost {
    fn granted(&self, _capability: &str) -> bool {
        self.grant
    }

    fn call(&mut self, capability: &str, _args: &[Value]) -> Result<Value, HostError> {
        Ok(Value::List(vec![
            Value::Symbol("ok".to_string()),
            Value::String(format!("{capability}:result")),
        ]))
    }
}

#[test]
fn calls_a_capability_through_the_host_bridge() {
    let generated = generated();
    let mut host = TestHost { grant: true };
    let args = [
        Value::String("signal:avery".to_string()),
        Value::Nil,
        Value::Int(50),
    ];

    let result = generated
        .catalog
        .call(&mut host, "soil:records/scan", &args)
        .expect("host call");
    assert_eq!(
        result,
        Value::List(vec![
            Value::Symbol("ok".to_string()),
            Value::String("soil:records/scan:result".to_string())
        ])
    );
}

#[test]
fn calls_a_generated_form() {
    let generated = generated();
    let mut host = TestHost { grant: true };
    let form = river_eval::sexp::parse_all(r#"(soil:records/scan "signal:avery" nil 50)"#)
        .expect("parse form")
        .remove(0);

    let result = call_form(&generated.catalog, &mut host, &form).expect("call form");
    assert!(matches!(result, Value::List(_)));
}

#[test]
fn a_denied_capability_does_not_reach_the_host() {
    let generated = generated();
    let mut host = TestHost { grant: false };
    let args = [
        Value::String("signal:avery".to_string()),
        Value::Nil,
        Value::Int(50),
    ];

    let error = generated
        .catalog
        .call(&mut host, "soil:records/scan", &args)
        .expect_err("denied");
    assert!(error.to_string().contains("denied"));
}

#[test]
fn arity_is_checked_before_the_host() {
    let generated = generated();
    let mut host = TestHost { grant: true };

    let error = generated
        .catalog
        .call(
            &mut host,
            "soil:records/scan",
            &[Value::String("x".to_string())],
        )
        .expect_err("arity");
    assert!(error.to_string().contains("expects 3 arguments"));
}
