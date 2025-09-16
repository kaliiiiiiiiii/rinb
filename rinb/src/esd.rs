use std::{
	env,
	fs::{self, create_dir_all},
	num::NonZeroUsize,
	path::PathBuf,
	thread,
};

use anyhow::{Error, Ok, Result, anyhow};
use hex::ToHex;
use uuid::Uuid;

use wimlib::{
	CompressionType, ExportFlags, ExtractFlags, Image, ImageIndex, OpenFlags, Wim, WimInfo, WimLib,
	WriteFlags, string::TStr, tstr,
};

use crate::utils::ExpectEqual;
pub struct EsdFile<'a> {
	pub path: &'a PathBuf,
	pub wiml: WimLib,
	pub info: WimInfo,
	pub wim: Wim,
	pub n_threads: u32,
}

impl<'a> EsdFile<'a> {
	pub fn new(path: &'a PathBuf) -> Result<Self, Error> {
		let n_threads = thread::available_parallelism()
			.unwrap_or(NonZeroUsize::new(8).unwrap())
			.get() as u32;
		let wiml = WimLib::default();
		let wim = wiml.open_wim(&TStr::from_path(path).unwrap(), OpenFlags::empty())?;
		let info = wim.info();
		return Ok(Self {
			path: path,
			wiml: wiml,
			info: info,
			wim: wim,
			n_threads: n_threads,
		});
	}

	pub fn xml(&self) -> Result<String, Error> {
		return Ok(self.wim.xml_data()?.to_string_lossy());
	}

	/// base image (index 1)
	pub fn base(&self) -> Result<Image<'_>, Error> {
		let base_image: Image<'_> = self.wim.select_image(ImageIndex::new(1).unwrap());
		let name = base_image.property(tstr!("NAME")).unwrap();
		name.expect_equal(
			tstr!("Windows Setup Media"),
			"Unexpected image name at index 1",
		)?;
		Ok(base_image)
	}

	// winPE image (index 2)
	pub fn win_pe(&self) -> Result<Wim, Error> {
		let boot_pe = self.wiml.create_new_wim(wimlib::CompressionType::Lzx)?;
		boot_pe.set_output_chunk_size(32 * 1024)?; // 32k, see https://github.com/ebiggers/wimlib/blob/e59d1de0f439d91065df7c47f647f546728e6a24/src/wim.c#L48-L83

		let win_pe = self.wim.select_image(ImageIndex::new(2).unwrap());
		let (name, descr, edition) = (
			win_pe.property(tstr!("NAME")).unwrap(),
			win_pe.property(tstr!("DESCRIPTION")).unwrap(),
			win_pe.property(tstr!("WINDOWS/EDITIONID")).unwrap(),
		);
		let flag = win_pe.property(tstr!("FLAGS")).unwrap();
		flag.expect_equal(tstr!("9"), "Expected image at index 2 to be WindowsPE")?;
		edition.expect_equal(
			tstr!("WindowsPE"),
			"Expected image at index 3 to be WindowsPE based",
		)?;

		win_pe.export(&boot_pe, Some(name), Some(descr), ExportFlags::BOOT)?;
		Ok(boot_pe)
	}

	// boot.wim image (index 3)
	pub fn boot(&self) -> Result<Wim, Error> {
		let boot_wim = self.wiml.create_new_wim(wimlib::CompressionType::Lzx)?;
		boot_wim.set_output_chunk_size(32 * 1024)?; // 32k, see https://github.com/ebiggers/wimlib/blob/e59d1de0f439d91065df7c47f647f546728e6a24/src/wim.c#L48-L83

		let win_setup = self.wim.select_image(ImageIndex::new(3).unwrap());
		let (name, descr, edition) = (
			win_setup.property(tstr!("NAME")).unwrap(),
			win_setup.property(tstr!("DESCRIPTION")).unwrap(),
			win_setup.property(tstr!("WINDOWS/EDITIONID")).unwrap(),
		);
		let flag = win_setup.property(tstr!("FLAGS")).unwrap();
		flag.expect_equal(tstr!("2"), "Expected image at index 3 to be Windows Setup")?;
		edition.expect_equal(
			tstr!("WindowsPE"),
			"Expected image at index 3 to be WindowsPE based",
		)?;
		win_setup.export(&boot_wim, Some(name), Some(descr), ExportFlags::BOOT)?;
		Ok(boot_wim)
	}

	// find install.esd based on edition
	pub fn install(&self, edition: &str) -> Result<Option<Image<'_>>, Error> {
		// 0 to image_count: add to install.esd for image which matches EDITIONID
		let mut install_esd: Option<Image> = None;
		for index in 4..=self.info.image_count {
			let wim = self.wim.select_image(ImageIndex::new(index).unwrap());

			// only add images where editionID matches
			let edition_got = wim.property(tstr!("WINDOWS/EDITIONID")).unwrap();
			if edition_got.to_str() == edition {
				if install_esd.is_some() {
					return Err(anyhow!(
						"Multiple install images matching selected edition ({edition}) found"
					));
				}
				install_esd = Some(wim);
			}
		}
		Ok(install_esd)
	}

	pub fn write(&self, wim: &Image, path: &PathBuf, max_file_size: &u64) -> Result<(), Error> {
		wim.write(
			&TStr::from_path(path).unwrap(),
			WriteFlags::empty(),
			self.n_threads,
		)?;

		// split wim if needed
		if &path.metadata()?.len() > max_file_size {
			let tmppath: &PathBuf = &env::temp_dir().join(format!(
				"rinb_tmp_file_{}.wim",
				Uuid::new_v4().encode_hex::<String>()
			));

			let wim = self
				.wiml
				.open_wim(&TStr::from_path(&path).unwrap(), OpenFlags::empty())?;

			// create new non-solid wim (WIMs inside esd are solid by default)
			// solid wims cannot be split
			// windows (probably) doesn't support lzms + non-solid, nor can wimlib convert to non-solid lzms
			// => must use slow (compression) lzx
			wim.set_output_compression_type(CompressionType::Lzx)?; // non-solid
			wim.set_output_chunk_size(32 * 1024)?; // 32k chunk size
			wim.select_all_images().write(
				&TStr::from_path(&tmppath).unwrap(),
				WriteFlags::empty(),
				self.n_threads,
			)?;

			// split new wim (ensuring cleanup)
			let result = (|| -> Result<()> {
				drop(wim);
				fs::remove_file(&path)?;
				// non-solid wim is now at tmppath
				let wim = self
					.wiml
					.open_wim(&TStr::from_path(&tmppath).unwrap(), OpenFlags::empty())?;
				// ~90% of max size to allow for some padding, ~3.5 MiB. on Win11 5.4 GiB observed
				wim.split(
					&TStr::from_path(&path.with_extension("swm")).unwrap(),
					(*max_file_size * 9) / 10,
					WriteFlags::empty(),
				)?;
				drop(wim);
				Ok(())
			})();
			// ensure cleanup
			fs::remove_file(&tmppath)?;
			result?;
		}
		Ok(())
	}

	pub fn install_dir(
		&self,
		target_dir: &PathBuf,
		edition: &str,
		max_file_size: &u64,
	) -> Result<(), Error> {
		create_dir_all(target_dir.join("sources"))?;

		let install_esd_path = target_dir.join("sources/install.esd");
		let boot_wim_path = target_dir.join("sources/boot.wim");

		// write boot.wim
		let boot_wim = self.boot()?;
		// let boot_wim = self.win_pe()?; // write win_pe for testing instead

		self.write(&boot_wim.select_all_images(), &boot_wim_path, max_file_size)?;

		// write install.esd to dism
		let install_esd = match self.install(edition)? {
			Some(esd) => esd,
			None => return Err(anyhow!("install.esd not found in image")),
		};
		self.write(&install_esd, &install_esd_path, max_file_size)?;

		// extract base image
		let base_image = self.base()?;
		let extract_flags = ExtractFlags::STRICT_ACLS
				// ExtractFlags::NTFS |
				| ExtractFlags::STRICT_GLOB
				| ExtractFlags::STRICT_SYMLINKS
				| ExtractFlags::STRICT_SHORT_NAMES;
		base_image.extract(&TStr::from_path(target_dir).unwrap(), extract_flags)?;
		Ok(())
	}
}
