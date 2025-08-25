mod config;
use config::Config;
mod esd_downloader;
use esd_downloader::WinEsdDownloader;
use wimlib::{string::TStr, OpenFlags, WimLib};

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

    let esd = downloader.download(&config.lang, &config.editon, config.arch.as_str())?;

    println!(
        "ESD file downloaded to {}",
        esd.display()
    );

    /* let dism = ESD::new(
        tmp_esd.path().to_str().unwrap().to_owned(),
        false,       // as_esd
        Some(1),     // index
        None,        // image_name
        true,        // as_readonly
        None,        // mountPath
        false,       // commitOnDispose
    ); */

    let wiml = WimLib::default();
    
    let wimf = wiml.open_wim(&TStr::from_path(esd).unwrap(), OpenFlags::WRITE_ACCESS)?;
    let xml = wimf.xml_data()?;
    let xml_str = xml.to_string()?;
    print!("{}",xml_str);

    // tmp_esd.close().unwrap();

    Ok(())
}
