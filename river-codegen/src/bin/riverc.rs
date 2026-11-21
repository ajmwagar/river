use std::path::PathBuf;

use anyhow::{bail, Result};

fn main() -> Result<()> {
    let args = std::env::args().skip(1);
    let mut lisp = false;
    let mut input = None;
    let mut output = None;
    for arg in args {
        match arg.as_str() {
            "--lisp" => lisp = true,
            "-h" | "--help" => {
                println!("usage: riverc [--lisp] INPUT.river [OUTPUT.wasm]");
                return Ok(());
            }
            other => {
                if input.is_none() {
                    input = Some(PathBuf::from(other));
                } else if output.is_none() {
                    output = Some(PathBuf::from(other));
                } else {
                    bail!("usage: riverc [--lisp] INPUT.river [OUTPUT.wasm]");
                }
            }
        }
    }
    let Some(input) = input else {
        bail!("usage: riverc [--lisp] INPUT.river [OUTPUT.wasm]");
    };
    let output = output.unwrap_or_else(|| PathBuf::from(format!("{}.wasm", input.display())));
    let source = std::fs::read_to_string(&input)?;
    let wasm = if lisp {
        river_codegen::lisp::compile_source(&source)?.wasm
    } else {
        let program = river_codegen::parse(&source)?;
        river_codegen::compile(&program)?
    };
    std::fs::write(&output, wasm)?;
    println!("{}", output.display());
    Ok(())
}
