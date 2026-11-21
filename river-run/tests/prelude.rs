use std::path::Path;

use river_eval::sexp::Value;
use river_run::Runtime;

fn runtime() -> Runtime {
    let mut runtime = Runtime::new(Box::new(river_run::FakeHost::new()));
    runtime
        .load_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("../prelude.river"))
        .unwrap();
    runtime
}

#[test]
fn map_filter_and_any_use_named_functions() {
    let mut runtime = runtime();
    let result = runtime
        .load(
            r#"
            (defun inc (n) (+ n 1))
            (defun even? (n) (or (= n 2) (= n 4)))
            (defun over-two? (n) (> n 2))
            (list (map (quote inc) (list 1 2 3))
                  (filter (quote even?) (list 1 2 3 4))
                  (any? (quote over-two?) (list 1 2 3)))
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::List(vec![Value::Int(2), Value::Int(3), Value::Int(4)]),
            Value::List(vec![Value::Int(2), Value::Int(4)]),
            Value::Bool(true),
        ])
    );
}

#[test]
fn json_get_in_and_string_contains() {
    let mut runtime = runtime();
    let result = runtime
        .load(
            r#"
            (let ((body (json-decode "{\"value\":{\"contact\":\"good\"}}")))
              (list (json-get-in body (list :value :contact))
                    (string-contains? "PineTime watch" "watch")
                    (rest (list 1))))
            "#,
        )
        .unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::String("good".to_string()),
            Value::Bool(true),
            Value::Nil,
        ])
    );
}
