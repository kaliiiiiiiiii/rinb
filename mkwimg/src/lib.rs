use anyhow::{Error, Result};
use std::{
	cell::Cell,
	fmt::Debug,
	fs::{self, File},
	path::Path,
};

use clap::ValueEnum;

use fatfs::{FatType, FileSystem, FormatVolumeOptions, FsOptions, ReadWriteSeek, format_volume};
use gpt::{
	GptConfig, disk::LogicalBlockSize, mbr::ProtectiveMBR, partition::Partition, partition_types,
};
use rdisk::vhd::VhdImage;

mod part;
use part::SPartition;
mod utils;

use utils::{VhdStream, dir2fat, dir2fatsize};

#[derive(ValueEnum, Debug, Clone)]
#[clap(rename_all = "kebab_case")]
pub enum PackType {
	VHD,
	IMG,
}
trait DReadWriteSeek: ReadWriteSeek + Debug {}
impl<T: ReadWriteSeek + Debug> DReadWriteSeek for T {}

// packs an installation dir to out as PackType
pub fn pack(dir: &Path, out: &Path, o_type: PackType) -> Result<(), Error> {
	if out.exists() {
		fs::remove_file(out)?;
	}
	// let mut img_file = File::create(out)?;

	let part_align = 1 * 1024 * 1024; // 1MiB
	let block_size = 512;
	let lb_size = LogicalBlockSize::Lb512;

	let efip = SPartition {
		name: format!("efi"),
		ptype: partition_types::BASIC, // windows cannot find the installation media if partition_types::EFI is used
		size: dir2fatsize(dir)?,
		flags: 0,
		id: Cell::new(None),
		startb: Cell::new(None),
		endb: Cell::new(None),
		align: Some(part_align / block_size),
	};
	let spartitions: Vec<&SPartition> = vec![&efip];

	let est_size: u64 = spartitions.iter().map(|p| p.size).sum::<u64>()
		+ (spartitions.len() as u64 * part_align) * 2
		+ (2 * 1024 * 1024);
	if out.exists() {
		fs::remove_file(out)?;
	}
	// vhd + 1MiB buffer
	let mut img_file: Box<dyn DReadWriteSeek>;
	match o_type {
		PackType::VHD => {
			let img = VhdImage::create_fixed(out.to_string_lossy(), est_size + 1024 * 1024)
				.map_err(|e| anyhow::anyhow!(e))?;
			img_file = Box::new(VhdStream::new(img));
		}
		PackType::IMG => {
			let file = File::create(out)?;
			file.set_len(est_size)?; // 4GiB
			img_file = Box::new(file);
		}
	}

	let mbr = ProtectiveMBR::with_lb_size(
		u32::try_from((est_size / block_size) - 1).unwrap_or(0xFF_FF_FF_FF),
	);
	mbr.overwrite_lba0(&mut img_file)?;

	let mut disk = GptConfig::new()
		.writable(true)
		.logical_block_size(lb_size)
		.create(out)
		.expect("failed to open disk");

	// create partitions
	for spart in spartitions.iter() {
		let id = disk.add_partition(
			&spart.name,
			spart.size,
			spart.ptype.clone(),
			spart.flags,
			spart.align,
		)?;
		spart.id.set(Some(id));
	}

	// find partition ofsets
	let device: File;
	{
		let partitions: std::collections::BTreeMap<_, Partition> = disk.partitions().clone();
		device = disk.write()?;

		let mut partc: usize = 0;

		for (key, partition) in partitions {
			if let Some(spartition) = spartitions.iter().find(|p| p.id.get() == Some(key)) {
				let startb = partition.bytes_start(lb_size)?;
				spartition.startb.set(Some(startb));
				let endb = partition.bytes_len(lb_size)? + startb;
				spartition.endb.set(Some(endb));
				partc += 1
			}
		}

		// we expect all partitions to be found
		assert_eq!(partc, spartitions.len());
	}

	// format and write to efi
	{
		let mut fdisk = efip.fdisk(&device)?;
		let fmt_opts = FormatVolumeOptions::new()
			.fat_type(FatType::Fat32)
			.volume_label(*b"System     ")
			.volume_id(0x12345678);
		format_volume(fdisk, fmt_opts)?;

		fdisk = efip.fdisk(&device)?;
		let fsf = FileSystem::new(fdisk, FsOptions::new())?;
		let root = fsf.root_dir();

		// fails with blank blue screen
		// boots when autorun.inf (referencing Setup.exe) is removed
		// then fails with:
		// A media driver your computer needs is missing. This could be a DVD, USB or Hard disk driver. If you have a CD, DVD, or USB flash drive with the driver on it, please insert it now.
		// Note: If the installation media for Windows is in the DVD drive or on a USB drive, you can safely remove it for this step.
		// Next:
		// No device drivers were found. Make sure that the installation media contains the correct drivers, and then click OK

		// => doesn't find setup.exe because it's within hidden system efi partition?

		// in winpe, fails with
		// (X:\$windows~bt\Windows). Windows PE cannot start because the actual SYSTEMROOT directory (X:\windows) is different from the configured one This can be configured from dism.exe with the /set-targetpath command.
		dir2fat(&root, dir)?;
	}
	Ok(())
}
