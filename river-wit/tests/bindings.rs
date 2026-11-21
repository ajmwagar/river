use std::path::Path;

use river_wit::generate;

fn generated() -> river_wit::Generated {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../soil/wit/soil-records.wit");
    generate(path).expect("generate soil:records")
}

#[test]
fn emits_type_definitions_and_capability_metadata() {
    let generated = generated();
    let text = &generated.sexp;

    assert!(text.contains("(define-package \"soil:records@0.1.0\""));
    assert!(text.contains("(define-record soil:records/entry"));
    assert!(text.contains("(define-enum soil:records/error"));
    assert!(text.contains("(define-capability soil:records/scan"));
    assert!(text.contains(":interface \"soil:records\""));
    assert!(text.contains(":function \"scan\""));
    assert!(text.contains(":grant \"soil:records\""));
    assert!(generated.capabilities >= 8);
    assert!(generated.types >= 8);
}

#[test]
fn emits_a_generic_host_call_stub() {
    let generated = generated();
    let text = &generated.sexp;

    assert!(text.contains("(defun soil:records/scan (collection cursor limit)"));
    assert!(text.contains("(host-call \"soil:records\" \"scan\" (list collection cursor limit))"));
}

#[test]
fn emits_a_stub_for_every_function() {
    let generated = generated();
    for function in ["get", "scan", "upsert", "delete", "changes", "apply"] {
        assert!(
            generated
                .sexp
                .contains(&format!("(define-capability soil:records/{function}")),
            "missing capability {function}"
        );
        assert!(
            generated
                .sexp
                .contains(&format!("(defun soil:records/{function}")),
            "missing stub {function}"
        );
    }
}
