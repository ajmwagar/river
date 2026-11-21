use wasmtime::{Engine, Instance, Module, Store, Val};

fn compiled() -> river_codegen::lisp::Compiled {
    let mut source = r#"
        (defun caar (x) (car (car x)))
        (defun cadr (x) (car (cdr x)))
        (defun cdar (x) (cdr (car x)))
        (defun caddr (x) (car (cdr (cdr x))))
        (defun cadar (x) (car (cdr (car x))))
        (defun caddar (x) (car (cdr (cdr (car x)))))
    "#
    .to_string();
    source.push_str(include_str!("../../river-run/tests/fixtures/jmc.lisp"));
    source.push_str(
        r#"
        (defun test-car ()
          (eval. (quote (car (cons (quote a) (quote ())))) (quote ())))
        (defun test-atom ()
          (eval. (quote (atom (quote a))) (quote ())))
        (defun test-eq ()
          (eval. (quote (eq (quote a) (quote a))) (quote ())))
        (defun test-quote-car ()
          (car (eval. (quote (quote (a b))) (quote ()))))
        (defun test-lambda ()
          (eval. (quote ((lambda (x) x) (quote a))) (quote ())))
        (defun test-label ()
          (eval. (quote ((label id (lambda (x) x)) (quote a))) (quote ())))
        "#,
    );
    river_codegen::lisp::compile_source(&source).expect("compile jmc.lisp")
}

fn call(compiled: &river_codegen::lisp::Compiled, name: &str) -> i64 {
    let engine = Engine::default();
    let module = Module::new(&engine, &compiled.wasm).expect("module");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("instance");
    let function = instance.get_func(&mut store, name).expect("function");
    let mut results = [Val::I64(0)];
    function.call(&mut store, &[], &mut results).expect("call");
    match results[0] {
        Val::I64(value) => value,
        other => panic!("unexpected result {other:?}"),
    }
}

#[test]
fn mccarthy_eval_runs_on_wasm() {
    let compiled = compiled();
    let a = compiled.symbols["a"];
    let t = compiled.symbols["t"];

    assert_eq!(call(&compiled, "test-car"), a);
    assert_eq!(call(&compiled, "test-atom"), t);
    assert_eq!(call(&compiled, "test-eq"), t);
    assert_eq!(call(&compiled, "test-quote-car"), a);
    assert_eq!(call(&compiled, "test-lambda"), a);
    assert_eq!(call(&compiled, "test-label"), a);
}
