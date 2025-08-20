mod config;
use config::Config;

use std::fs;
use std::error::Error;

use clap::Parser;



#[derive(Parser, Debug)]
#[command(version, about = "App with JSON config")]
struct Args {
    /// Path to config file
    #[arg(long)]
    config: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let data = fs::read_to_string(&args.config)?;
    let config: Config = serde_json5::from_str(&data)?;

    println!("Loaded config: {:?}", config);

    Ok(())
}
