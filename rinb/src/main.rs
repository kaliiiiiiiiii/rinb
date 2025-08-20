mod config;
use config::Config;
mod esd_downloader;
use esd_downloader::WinEsdDownloader;

use std::fs;

use clap::Parser;

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

    // Define the Windows version you want to download
    let edition = "Professional"; // Windows 11 Pro
    let architecture = "x64"; // 64-bit

    println!("Language: {}", config.lang);
    println!("Edition: {}", edition);
    println!("Architecture: {}", architecture);

    let tmp_esd = downloader.download_tmp(&config.lang, edition, architecture)?;

    println!("ESD file saved to: {}, deleting now", tmp_esd.path().display());
    tmp_esd.close().unwrap();

    Ok(())
}
