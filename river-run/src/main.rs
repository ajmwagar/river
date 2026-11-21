use std::path::PathBuf;

use anyhow::{bail, Result};
use river_eval::sexp::Value;
use river_run::{FakeHost, Runtime, SoilRecordsHost};

fn main() -> Result<()> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut call: Option<String> = Some("main".to_string());
    let mut records_root: Option<PathBuf> = None;
    let mut seed = false;
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
            "--records-root" => {
                records_root =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        anyhow::anyhow!("--records-root needs a path")
                    })?));
            }
            "--seed" => seed = true,
            "-h" | "--help" => {
                println!(
                    "usage: river-run [--call NAME | --no-call] [--records-root DIR] [--seed] FILE..."
                );
                return Ok(());
            }
            other => files.push(PathBuf::from(other)),
        }
    }

    if files.is_empty() {
        bail!("usage: river-run [--call NAME | --no-call] [--records-root DIR] [--seed] FILE...");
    }
    if seed && records_root.is_none() {
        bail!("--seed requires --records-root");
    }

    let host: Box<dyn river_run::Host> = if let Some(root) = records_root {
        let mut host = SoilRecordsHost::open(&root)?;
        if seed {
            host.seed_instinct()?;
        }
        Box::new(host)
    } else {
        Box::new(FakeHost::new())
    };

    let mut runtime = Runtime::new(host);
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
