use anyhow::{Error, Result};
use std::{
	fs,
	path::{Path, PathBuf},
	time::Instant,
};

use clap::{Parser, ValueEnum};
use serde_json;
use serde_json5;

use mkwimg::{PackType, pack};

use rinb::config::Config;

use rinb::esd_downloader::WinEsdDownloader;

use rinb::esd::EsdFile;

#[derive(ValueEnum, Debug, Clone)]
#[clap(rename_all = "kebab_case")]
enum OutType {
	ISO,
	VHD,
	IMG,
}

#[derive(Parser, Debug)]
#[command(version, about = "Builds a customized windows installation")]
struct Args {
	/// Path to config file, {path}.lock{extension} will be used if it exists.
	#[arg(long, default_value = "rinb.json", alias = "c")]
	config: String,
	#[arg(long, default_value = "out/devwin.iso", alias = "o")]
	out: String,
	#[arg(long = "type", default_value = "iso", alias = "t")]
	o_type: OutType,
	#[arg(long, default_value = "./.rinbcache/esd_cache", alias = "cc")]
	cache_path: String,
}

impl Args {
	fn lock_path(&self) -> PathBuf {
		let original = PathBuf::from(self.config.clone());
		let parent = original.parent().unwrap_or_else(|| Path::new(""));

		let file_name = original.file_name().unwrap_or_default().to_string_lossy();

		// Insert "-lock" before the first dot, or at the end if no dot exists
		let mut parts = file_name.splitn(2, '.');
		let base = parts.next().unwrap_or("");
		let rest = parts.next();

		let mut new_name = format!("{}.lock", base);
		if let Some(rest) = rest {
			new_name.push('.');
			new_name.push_str(rest);
		}

		parent.join(new_name)
	}
}

fn main() -> Result<(), Error> {
	let args = Args::parse();
	let mut config: Config;

	// identify cfg_path to use (lock or regular)
	let cfg_path: PathBuf;
	let lock_path = &args.lock_path();
	if lock_path.exists() {
		cfg_path = lock_path.to_path_buf()
	} else {
		cfg_path = PathBuf::from(&args.config)
	}
	{
		let data = fs::read_to_string(cfg_path)?;
		config = serde_json5::from_str(&data)?;
	}

	// download esd image
	let (esd, sha1size, url): (PathBuf, String, String);
	{
		let downloader = WinEsdDownloader::new(args.cache_path)?;
		(esd, sha1size, url) = downloader.download(&config)?;
	}

	// lock esd for url & sha1size
	config.url = Some(url);
	config.sha1size = Some(sha1size);
	{
		let data = serde_json::to_string_pretty(&config)?;
		fs::write(lock_path, data)?
	}

	//let tmp_dir = &TmpDir::new()?;
	//let tmp_dir_path = &tmp_dir.path;
	let tmp_dir_path = PathBuf::from(&args.out).parent().unwrap().join("isodir"); // for debugging

	println!("Starting to build");
	let now = Instant::now();

	// create install dir from esd
	let esdf = EsdFile::new(&esd)?;
	// println!("{}", esdf.xml()?);
	esdf.install_dir(&tmp_dir_path, &config.edition)?;

	let outp = Path::new(&args.out);
	match args.o_type {
		OutType::ISO => pack(&tmp_dir_path, outp, PackType::ISO)?,
		OutType::VHD => pack(&tmp_dir_path, outp, PackType::VHD)?,
		OutType::IMG => pack(&tmp_dir_path, outp, PackType::IMG)?,
	}
	println!("Building took {:.2?}", now.elapsed());
	Ok(())
}
