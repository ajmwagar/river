use std::path::PathBuf;

use anyhow::{bail, Result};
use river_wit::generate::generate;

fn main() -> Result<()> {
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => {
                output = Some(
                    args.next()
                        .map(PathBuf::from)
                        .ok_or_else(|| anyhow::anyhow!("--out needs a path"))?,
                );
            }
            "-h" | "--help" => {
                println!("usage: river-wit <wit-file-or-dir> [--out FILE]");
                return Ok(());
            }
            other => {
                if input.is_some() {
                    bail!("unexpected argument `{other}`");
                }
                input = Some(PathBuf::from(other));
            }
        }
    }

    let input = input.ok_or_else(|| anyhow::anyhow!("missing WIT path"))?;
    let generated = generate(&input)?;

    if let Some(path) = output {
        std::fs::write(&path, &generated.sexp)?;
        eprintln!(
            "wrote {} ({} capabilities, {} types) to {}",
            generated.catalog.package,
            generated.catalog.capabilities.len(),
            generated.catalog.types.len(),
            path.display()
        );
    } else {
        print!("{}", generated.sexp);
    }

    Ok(())
}
