use std::path::Path;

use river_eval::sexp::Value;
use river_run::Runtime;

fn runtime() -> Runtime {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut runtime = Runtime::new(Box::new(river_run::FakeHost::new()));
    runtime.load_file(dir.join("../prelude.river")).unwrap();
    runtime
        .load_file(dir.join("tests/fixtures/jmc.lisp"))
        .unwrap();
    runtime
}

#[test]
fn mccarthy_eval_runs_in_the_interpreter() {
    let mut runtime = runtime();
    assert_eq!(
        runtime
            .load("(eval. '(car (cons (quote a) (quote ()))) '())")
            .unwrap(),
        Value::Symbol("a".to_string())
    );
    assert_eq!(
        runtime.load("(eval. '(atom (quote a)) '())").unwrap(),
        Value::Symbol("t".to_string())
    );
    assert_eq!(
        runtime.load("(eval. '(quote (a b)) '())").unwrap(),
        Value::List(vec![
            Value::Symbol("a".to_string()),
            Value::Symbol("b".to_string())
        ])
    );
}
