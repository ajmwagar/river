use std::path::PathBuf;

use anyhow::{bail, Result};
use river_eval::sexp::Value;
use river_run::{FakeHost, Host, HostChain, Runtime, SoilRecordsHost, SystemTimeHost};

fn main() -> Result<()> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut prelude: Option<PathBuf> = None;
    let mut call: Option<String> = Some("main".to_string());
    let mut records_root: Option<PathBuf> = None;
    let mut book = "river".to_string();
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
            "--prelude" => {
                prelude = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("--prelude needs a file"))?,
                ));
            }
            "--records-root" => {
                records_root =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        anyhow::anyhow!("--records-root needs a path")
                    })?));
            }
            "--book" => {
                book = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--book needs a name"))?;
            }
            "-h" | "--help" => {
                println!(
                    "usage: river-run [--call NAME | --no-call] [--prelude FILE] \
                     [--records-root DIR] [--book NAME] FILE..."
                );
                return Ok(());
            }
            other => files.push(PathBuf::from(other)),
        }
    }

    if files.is_empty() && prelude.is_none() {
        bail!(
            "usage: river-run [--call NAME | --no-call] [--prelude FILE] \
             [--records-root DIR] [--book NAME] FILE..."
        );
    }

    let mut hosts: Vec<Box<dyn Host>> = vec![Box::new(SystemTimeHost)];
    match records_root {
        Some(root) => hosts.push(Box::new(SoilRecordsHost::open(&root, book)?)),
        None => hosts.push(Box::new(FakeHost::new())),
    }
    let mut runtime = Runtime::new(Box::new(HostChain::new(hosts)));

    if let Some(prelude) = prelude {
        runtime.load_file(prelude)?;
    }
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
