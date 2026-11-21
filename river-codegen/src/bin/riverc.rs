use std::path::PathBuf;

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(input) = args.next() else {
        bail!("usage: riverc INPUT.river [OUTPUT.wasm]");
    };
    let output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("{input}.wasm")));
    if args.next().is_some() {
        bail!("usage: riverc INPUT.river [OUTPUT.wasm]");
    }
    let source = std::fs::read_to_string(&input)?;
    let program = river_codegen::parse(&source)?;
    let wasm = river_codegen::compile(&program)?;
    std::fs::write(&output, wasm)?;
    println!("{}", output.display());
    Ok(())
}
