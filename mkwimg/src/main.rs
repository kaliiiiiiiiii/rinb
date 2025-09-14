use anyhow::{Error, Ok, Result};
use mkwimg::{PackType, pack};
use std::path::Path;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about = "App with JSON config")]
struct Args {
	/// Path to installation media directory
	#[arg(long, default_value = "out/isodir", alias = "d")]
	dir: String,
	#[arg(long, default_value = "out/devwin.img", alias = "o")]
	out: String,
	#[arg(long = "type", default_value = "vhd", alias = "t")]
	o_type: PackType,
}

fn main() -> Result<(), Error> {
	let args = Args::parse();
	pack(&Path::new(&args.dir), &Path::new(&args.out), args.o_type)?;
	Ok(())
}
