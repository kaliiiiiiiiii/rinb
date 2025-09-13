mod download;

mod config;
use config::Config;

mod esd_downloader;
use esd_downloader::WinEsdDownloader;

/*
mod hadris_pack;
use hadris_pack::pack; */

mod utils;
use utils::{ExpectEqual, TmpDir, img_info, mk_pb, progress_callback};

use anyhow::{Error, Result};

use std::{
	fs::{self, create_dir_all},
	num::NonZeroUsize,
	path::{Path, PathBuf},
	thread,
};
use wimlib::{
	ExportFlags, ExtractFlags, ImageIndex, OpenFlags, WimLib, WriteFlags, string::TStr, tstr,
};

use clap::Parser;
use serde_json;
use serde_json5;

#[derive(Parser, Debug)]
#[command(version, about = "App with JSON config")]
struct Args {
	/// Path to config file, {path}.lock{extension} will be used if it exists.
	#[arg(long, default_value = "rinb.json", alias = "c")]
	config: String,
	#[arg(long, default_value = "out/devwin.iso", alias = "o")]
	out: String,
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
	let tmp_dir_path = PathBuf::from(args.out).parent().unwrap().join("isodir"); // for debugging

	// using wimlib
	{
		let wiml = WimLib::default();

		let n_threads = thread::available_parallelism()
			.unwrap_or(NonZeroUsize::new(8).unwrap())
			.get() as u32;

		let wimf = wiml.open_wim(&TStr::from_path(esd).unwrap(), OpenFlags::empty())?;
		let info = wimf.info();
		println!("{:#?}", wimf.xml_data());

		create_dir_all(tmp_dir_path.join("sources"))?;

		// 1: get base image
		let base_image = wimf.select_image(ImageIndex::new(1).unwrap());
		let (name, _, _) = img_info(&base_image);
		name.expect_equal(
			tstr!("Windows Setup Media"),
			"Unexpected image name at index 1",
		)?;

		// create boot.wim
		{
			let boot_wim_path = tmp_dir_path.join("sources/boot.wim");
			let mut boot_wim = wiml.create_new_wim(wimlib::CompressionType::Lzx)?;
			// boot_wim.set_output_chunk_size(128 * 1024)?; // 128k
			boot_wim.set_output_chunk_size(32 * 1024)?; // 32k, see https://github.com/ebiggers/wimlib/blob/e59d1de0f439d91065df7c47f647f546728e6a24/src/wim.c#L48-L83

			// 2: add Windows PE (no setup) to boot.wim // TODO: do we even need this? (probably not - to test)
			// https://www.ntlite.com/community/index.php?threads/edit-image-name-description-and-flags.3714/post-43298
			let win_pe = wimf.select_image(ImageIndex::new(2).unwrap());
			let (name, descr, edition) = img_info(&win_pe);
			// TODO: Windows PE (no setup) should be flag 9
			edition.expect_equal(tstr!("WindowsPE"), "Unexpected image at index 2")?;
			win_pe.export(&boot_wim, Some(name), Some(descr), ExportFlags::empty())?;

			// 3: add Windows PE (with setup) to boot.wim
			let win_setup = wimf.select_image(ImageIndex::new(3).unwrap());
			let (name, descr, edition) = img_info(&win_setup);
			// TODO: Windows PE (with setup) should be flag 2 (bootable)
			edition.expect_equal(tstr!("WindowsPE"), "Unexpected image at index 3")?;
			win_setup.export(&boot_wim, Some(name), Some(descr), ExportFlags::BOOT)?;

			// write boot.wim to disk
			let bar_msg = format!("compressing and writing boot.wim\n");
			let pb = mk_pb(&bar_msg);
			boot_wim.register_progress_callback(move |msg| {
				progress_callback(msg, &pb, bar_msg.clone())
			});

			boot_wim.select_all_images().write(
				&TStr::from_path(boot_wim_path).unwrap(),
				WriteFlags::empty(),
				n_threads,
			)?;
		}

		// create install.esd //TODO: use install_wim.write instead
		{
			let install_esd_path = tmp_dir_path.join("sources/install.esd");
			let mut install_esd = wiml.create_new_wim(wimlib::CompressionType::Lzms)?;
			// install_esd.register_progress_callback(progress_callback);
			install_esd.set_output_chunk_size(128 * 1024)?; // 128k

			// 0 to image_count: add to install.esd for image which matches EDITIONID
			let mut install_found = false;
			for index in 4..=info.image_count {
				let install_wim = wimf.select_image(ImageIndex::new(index).unwrap());

				// only  add images where editionID matches
				let (name, descr, edition) = img_info(&install_wim);
				if edition.to_str() == config.edition {
					if install_found {
						return Err(Error::msg(format!(
							"Multiple install images matching selected edition ({}) found",
							config.edition
						)));
					} else {
						install_found = true;
						install_wim.export(
							&install_esd,
							Some(name),
							Some(descr),
							ExportFlags::empty(),
						)?;
					}
				} else {
					// always export for testing
					install_wim.export(&install_esd, Some(name), Some(descr), ExportFlags::empty())?;
				}
			}
			if !install_found {
				return Err(Error::msg(format!(
					"No install images matching selected edition ({}) found",
					config.edition
				)));
			}

			// write install.esd to disk
			let bar_msg = format!("compressing and writing install.esd\n");
			let pb = mk_pb(&bar_msg);
			install_esd.register_progress_callback(move |msg| {
				progress_callback(msg, &pb, bar_msg.clone())
			});
			install_esd.select_all_images().write(
				&TStr::from_path(install_esd_path).unwrap(),
				WriteFlags::empty(),
				n_threads,
			)?;
		}

		{
			// extract base image to disk
			let extract_flags = ExtractFlags::STRICT_ACLS
				// ExtractFlags::NTFS |
				| ExtractFlags::STRICT_GLOB
				| ExtractFlags::STRICT_SYMLINKS
				| ExtractFlags::STRICT_SHORT_NAMES;
			// TODO: add progress bar for extracting.
			base_image.extract(&TStr::from_path(&tmp_dir_path).unwrap(), extract_flags)?;
		}
		drop(wiml);
	}

	// pack(&tmp_dir.path, &PathBuf::from(args.out))?;

	Ok(())
}
