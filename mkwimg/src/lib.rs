use std::{
	fs::{self, File}, io::Seek, path::Path
};
use anyhow::{Error, Ok, Result};

use gpt::{mbr::ProtectiveMBR, GptConfig, partition::Partition, disk::LogicalBlockSize};

pub fn pack(dir: &Path, out: &Path) -> Result<(), Error> {
	if out.exists() {
		fs::remove_file(out)?;
	}
	let mut img_file = File::create(out)?;

	let s_efi = 300 * 1024 * 1024;
	let s_msr = 16 * 1024 * 1024;
	let s_primary = 2 * 1024 * 1024 * 1024;
	let s_recovery = 300 * 1024 * 1024;

	let n_partitions = 4;
    let block_size = 512;
	let part_align = 1 * 1024 * 1024;
	let est_size: u64 = (s_efi + s_msr + s_primary + s_recovery)
		+ (n_partitions * part_align) * 2
		+ (2 * 1024 * 1024);

	img_file.set_len(est_size)?; // 4GiB
	let mbr = ProtectiveMBR::with_lb_size(
		u32::try_from((est_size / block_size) - 1).unwrap_or(0xFF_FF_FF_FF),
	);
	mbr.overwrite_lba0(&mut img_file)?;

	let mut disk = GptConfig::new()
		.writable(true)
		.logical_block_size(gpt::disk::LogicalBlockSize::Lb512)
		.create(out)
		.expect("failed to open disk");

	// create partitions based on https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/oem-deployment-of-windows-desktop-editions-sample-scripts?view=windows-11&preserve-view=true#createpartitions-uefitxt
	// efi partition
	//TODO: format as fat32
	disk.add_partition("efi", s_efi, gpt::partition_types::EFI, 0, None)?;

	// 16MiB msr partition
	disk.add_partition(
		"msr",
		s_msr,
		gpt::partition_types::MICROSOFT_RESERVED,
		0,
		None,
	)?;

	//primary partition
	// TODO: format as ntfs
	disk.add_partition("Windows", s_primary, gpt::partition_types::BASIC, 0, None)?;

	// recovery partition
	// TODO: format as ntfs
	disk.add_partition(
		"Recovery",
		s_recovery,
		gpt::partition_types::BASIC,
		0x8000000000000001,
		None,
	)?;

	let partitions: std::collections::BTreeMap<_, Partition> = disk.partitions().clone();
	let mut device = disk.write()?;

    for (key, partition) in partitions {
        println!("{key}:{partition:#?}")
    }
    // device.seek(pos);

	// todo: apply based on https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/oem-deployment-of-windows-desktop-editions-sample-scripts?view=windows-11&preserve-view=true#applyimagebat
	Ok(())
}
