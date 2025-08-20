use std::{env, fs, path::PathBuf};
use schemars::schema_for;

include!("src/config.rs");

fn main() -> anyhow::Result<()> {
    let schema = schema_for!(Config);
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dest = manifest_dir.parent().unwrap().join("rinb_schema.json");

    fs::write(
        &dest,
        serde_json::to_string_pretty(&schema)?
    )?;

    println!("cargo:warning=Schema written to {}", dest.display());
    Ok(())
}
