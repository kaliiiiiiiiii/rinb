mod config;
use config::Config;
mod esd_downloader;
use esd_downloader::WinEsdDownloader;
use wimlib::{string::TStr, OpenFlags, WimLib};

use widestring::{error::NulError, U16CString};

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

pub fn tstr_from_str(s: &str) -> Result<Box<TStr>, NulError<u16>> {
    // Convert &str → UTF-16 nul-terminated CString
    let u16_cstring = U16CString::from_str(s)?;
    let boxed = u16_cstring.into_boxed_ucstr();
    // Safety: U16CStr and TStr are repr(transparent)
    Ok(unsafe { Box::from_raw(Box::into_raw(boxed) as *mut TStr) })
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
    
    let wimf = wiml.open_wim(&tstr_from_str(esd.to_str().unwrap()).unwrap(), OpenFlags::WRITE_ACCESS)?;
    let xml = wimf.xml_data()?;
    let xml_str = xml.to_string()?;
    print!("{}",xml_str);

    // tmp_esd.close().unwrap();

    Ok(())
}
