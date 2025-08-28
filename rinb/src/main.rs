mod config;
use config::Config;

mod esd_downloader;
use esd_downloader::WinEsdDownloader;

mod utils;
use utils::{ExpectEqual, TmpDir};

use anyhow::Error;
use std::{
	fs::{self, create_dir_all},
	num::NonZeroUsize,
	path::PathBuf,
	thread,
};
use wimlib::{
	ExportFlags, ExtractFlags, Image, ImageIndex, OpenFlags, WimLib, WriteFlags, string::TStr, tstr,
};

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

fn img_info<'a>(image: &'a Image<'a>) -> (&'a TStr, &'a TStr, &'a TStr) {
	let (name, descr, edition) = (
		image.property(tstr!("NAME")).unwrap(),
		image.property(tstr!("DESCRIPTION")).unwrap(),
		image
			.property(tstr!("WINDOWS/EDITIONID"))
			.unwrap_or(tstr!("")),
	);
	return (name, descr, edition);
}

fn main() -> Result<(), Error> {
	let args = Args::parse();
	let config: Config;
	{
		let data = fs::read_to_string(&args.config)?;
		config = serde_json5::from_str(&data)?;
	}

	let esd: PathBuf;
	{
		let downloader = WinEsdDownloader::new(args.cache_path)?;
		esd = downloader.download(&config.lang, &config.editon, config.arch.as_str())?;
	}

	println!("ESD file downloaded to {}", esd.display());

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

	let n_threads = thread::available_parallelism()
		.unwrap_or(NonZeroUsize::new(8).unwrap())
		.get() as u32;

	let wimf = wiml.open_wim(&TStr::from_path(esd).unwrap(), OpenFlags::empty())?;
	let info = wimf.info();

	let tmp_dir = TmpDir::new()?;
	create_dir_all(tmp_dir.path.join("sources"))?;

	// 1: get base image
	let base_image = wimf.select_image(ImageIndex::new(1).unwrap());
	let (name, _, _) = img_info(&base_image);
	name.expect_equal(
		tstr!("Windows Setup Media"),
		"Unexpected image name at index 1",
	)?;

	// create boot.wim
	{
		let boot_wim_path = tmp_dir.path.join("sources/boot.wim");
		let boot_wim = wiml.create_new_wim(wimlib::CompressionType::Lzx)?;
		boot_wim.set_output_chunk_size(32 * 1024)?; // 32k, see https://github.com/ebiggers/wimlib/blob/e59d1de0f439d91065df7c47f647f546728e6a24/src/wim.c#L48-L83

		// 2: add Windows PE to boot.wim
		let win_pe = wimf.select_image(ImageIndex::new(2).unwrap());
		let (name, descr, edition) = img_info(&win_pe);
		edition.expect_equal(tstr!("WindowsPE"), "Unexpected image at index 2")?;
		win_pe.export(&boot_wim, Some(name), Some(descr), ExportFlags::empty())?;

		// 3: add Windows Setup to boot.wim
		let win_setup = wimf.select_image(ImageIndex::new(3).unwrap());
		let (name, descr, edition) = img_info(&win_setup);
		edition.expect_equal(tstr!("WindowsPE"), "Unexpected image at index 3")?;
		win_setup.export(&boot_wim, Some(name), Some(descr), ExportFlags::BOOT)?;

		// writ boot.wim to disk
		boot_wim.select_all_images().write(
			&TStr::from_path(boot_wim_path).unwrap(),
			WriteFlags::empty(),
			n_threads,
		)?;
	}

	// create install.esd
	{
		let install_esd_path = tmp_dir.path.join("sources/install.esd");
		let install_esd = wiml.create_new_wim(wimlib::CompressionType::Lzms)?;
		install_esd.set_output_chunk_size(128 * 1024)?; // 128k

		// 0 to image_count: add to install.esd for image which matches EDITIONID
		let mut install_found = false;
		for index in 4..=info.image_count {
			let install_wim = wimf.select_image(ImageIndex::new(index).unwrap());

			// only  add images where editionID matches
			let (name, descr, edition) = img_info(&install_wim);
			if edition.to_str() == config.editon {
				if install_found {
					return Err(Error::msg(format!(
						"Multiple install images matching selected edition ({}) found",
						config.editon
					)));
				} else {
					install_found = true;
					install_wim.export(&install_esd, Some(name), Some(descr), ExportFlags::BOOT)?;
				}
			}
		}
		if !install_found {
			return Err(Error::msg(format!(
				"No install images matching selected edition ({}) found",
				config.editon
			)));
		}

		// write install.esd to disk
		install_esd.select_all_images().write(
			&TStr::from_path(install_esd_path).unwrap(),
			WriteFlags::RECOMPRESS,
			n_threads,
		)?;
	}

	// extract base image to disk
	let extract_flags = ExtractFlags::STRICT_ACLS
        // ExtractFlags::NTFS |
        | ExtractFlags::STRICT_GLOB
        | ExtractFlags::STRICT_SYMLINKS
        | ExtractFlags::STRICT_SHORT_NAMES;
	base_image.extract(&TStr::from_path(&tmp_dir.path).unwrap(), extract_flags)?;

	Ok(())
}
