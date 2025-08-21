mod config;
use config::Config;
mod esd_downloader;
use esd_downloader::WinEsdDownloader;
// mod wim;
// use wim::ESD;

use std::fs;

use clap::Parser;
use serde_json5;

#[derive(Parser, Debug)]
#[command(version, about = "App with JSON config")]
struct Args {
    /// Path to config file
    #[arg(long)]
    config: String,
    #[arg(long, default_value = "./.rinbcache/esd_cache")]
    cache_path: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let data = fs::read_to_string(&args.config)?;
    let config: Config = serde_json5::from_str(&data)?;

    let downloader = WinEsdDownloader::new(args.cache_path)?;

    let tmp_esd = downloader.download_tmp(&config.lang, &config.editon, config.arch.as_str())?;

    println!("ESD file saved to: {}, deleting now", tmp_esd.path().display());

     /* let dism = ESD::new(
        tmp_esd.path().to_str().unwrap().to_owned(),
        false,       // as_esd
        Some(1),     // index
        None,        // image_name
        true,        // as_readonly
        None,        // mountPath
        false,       // commitOnDispose
    ); */

    tmp_esd.close().unwrap();

    Ok(())
}
