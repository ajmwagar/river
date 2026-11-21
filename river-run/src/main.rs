use std::path::PathBuf;

use anyhow::{bail, Result};
use river_eval::sexp::Value;
use river_run::{FakeHost, Runtime};

fn main() -> Result<()> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut call: Option<String> = Some("main".to_string());
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--call" => {
                call = Some(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--call needs a name"))?,
                );
            }
            "--no-call" => call = None,
            "-h" | "--help" => {
                println!("usage: river-run [--call NAME | --no-call] FILE...");
                return Ok(());
            }
            other => files.push(PathBuf::from(other)),
        }
    }

    if files.is_empty() {
        bail!("usage: river-run [--call NAME | --no-call] FILE...");
    }

    let mut runtime = Runtime::new(Box::new(FakeHost::new()));
    let mut last = Value::Nil;

    for file in files {
        last = runtime.load_file(&file)?;
    }

    if let Some(name) = call {
        if runtime.has_function(&name) {
            let value = runtime.call(&name, &[])?;
            if !matches!(value, Value::Nil) {
                println!("{value}");
            }
        } else if !matches!(last, Value::Nil) {
            println!("{last}");
        }
    }

    Ok(())
}
