use std::path::Path;

use river_eval::sexp::Value;
use river_run::{Runtime, SoilRecordsHost};

#[test]
fn instinct_runs_on_the_in_process_records_host() {
    let root = tempfile::tempdir().unwrap();
    let mut host = SoilRecordsHost::open(root.path()).unwrap();
    host.seed_instinct().unwrap();

    let mut runtime = Runtime::new(Box::new(host));
    let generated =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../river-wit/generated/soil-records.river");
    // Mirrors `instinct/main.river`; kept local so this test does not depend
    // on a sibling checkout.
    let main = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/instinct.river");
    runtime.load_file(generated).unwrap();
    runtime.load_file(main).unwrap();

    assert_eq!(
        runtime.call("main", &[]).unwrap(),
        Value::String("worn".to_string())
    );
}
