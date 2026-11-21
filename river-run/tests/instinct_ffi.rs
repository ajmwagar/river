use std::path::Path;

use river_eval::sexp::Value;
use river_run::{HostChain, Runtime, SoilRecordsHost, SystemTimeHost};

#[test]
fn instinct_runs_as_river_over_the_records_wit() {
    let root = tempfile::tempdir().unwrap();
    let host = HostChain::new(vec![
        Box::new(SystemTimeHost),
        Box::new(SoilRecordsHost::open(root.path(), "river").unwrap()),
    ]);
    let mut runtime = Runtime::new(Box::new(host));

    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    runtime
        .load_file(dir.join("../river-wit/generated/soil-records.river"))
        .unwrap();
    runtime
        .load_file(dir.join("../river-wit/generated/soil-time.river"))
        .unwrap();
    runtime.load_file(dir.join("../prelude.river")).unwrap();
    runtime
        .load_file(dir.join("examples/seed_instinct.river"))
        .unwrap();
    runtime
        .load_file(dir.join("examples/instinct.river"))
        .unwrap();

    let result = runtime.call("main", &[]).unwrap();
    let text = result.to_string();
    assert!(text.contains("(:worn 0.9)"), "{text}");
    assert!(text.contains("(:worn 0.8)"), "{text}");
    assert!(text.contains("(:worn 0.85)"), "{text}");
    assert!(text.contains("(:in-use 0.85)"), "{text}");
    assert!(text.contains("(:present 0.9)"), "{text}");
    assert!(matches!(result, Value::List(_)));
}
