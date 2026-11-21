use std::path::Path;

use river_eval::sexp::Value;
use river_run::{FakeHost, Runtime};

fn generated_records() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../river-wit/generated/soil-records.river")
}

fn runtime_with_records(host: FakeHost) -> Runtime {
    let mut runtime = Runtime::new(Box::new(host));
    runtime
        .load_file(generated_records())
        .expect("load generated records bindings");
    runtime
        .load(
            r#"
            (defun main ()
              (let ((result (soil:records/scan "signal:avery" nil 50)))
                (if (ok? result)
                    (length (field (value result) :entries))
                    0)))
            "#,
        )
        .expect("load main");
    runtime
}

#[test]
fn evaluates_defun_let_and_arithmetic() {
    let mut runtime = Runtime::new(Box::new(FakeHost::new()));
    let result = runtime
        .load("(defun add (a b) (+ a b)) (add 2 3)")
        .expect("eval");
    assert_eq!(result, Value::Int(5));
}

#[test]
fn evaluates_cond_and_strings() {
    let mut runtime = Runtime::new(Box::new(FakeHost::new()));
    let result = runtime
        .load(
            r#"
            (defun label (n)
              (cond ((> n 10) "big")
                    ((> n 5) "medium")
                    (else "small")))
            (label 7)
            "#,
        )
        .expect("eval");
    assert_eq!(result, Value::String("medium".to_string()));
}

#[test]
fn calls_generated_records_stub_through_the_host() {
    let mut host = FakeHost::new();
    host.insert("signal:avery", "pine:1:hr", br#"{"bpm":61}"#);
    host.insert("signal:avery", "pine:1:motion", br#"{"moving":true}"#);
    let mut runtime = runtime_with_records(host);
    let result = runtime.call("main", &[]).expect("call main");
    assert_eq!(result, Value::Int(2));
}

#[test]
fn a_denied_capability_fails_before_the_host() {
    let mut runtime = runtime_with_records(FakeHost::new().deny_all());
    let error = runtime.call("main", &[]).expect_err("denied");
    assert!(format!("{error:#}").contains("denied"));
}

#[test]
fn a_narrow_grant_only_allows_the_named_function() {
    let host = FakeHost::new().grant("soil:records", "scan");
    let mut runtime = Runtime::new(Box::new(host));
    runtime
        .load_file(generated_records())
        .expect("load generated records bindings");

    let result = runtime
        .load(r#"(soil:records/scan "signal:avery" nil 50)"#)
        .expect("scan allowed");
    assert!(
        matches!(result.as_list().and_then(|items| items.first()), Some(Value::Symbol(tag)) if tag == "ok")
    );

    let error = runtime
        .load(
            r#"(soil:records/get (list (list :collection "signal:avery") (list :id "pine:1:hr")))"#,
        )
        .expect_err("get denied");
    assert!(format!("{error:#}").contains("denied"));
}
