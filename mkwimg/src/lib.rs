use anyhow::{Error, Ok, Result, anyhow};
use std::{
	cell::{Cell, RefCell},
	fs::{self, File},
	io::{Read, Seek, Write},
	path::Path,
};

use fscommon::StreamSlice;
use gpt::{
	GptConfig,
	disk::LogicalBlockSize,
	mbr::ProtectiveMBR,
	partition::Partition,
	partition_types::{self, Type as PType},
};

use fatfs::{FatType, FileSystem, FormatVolumeOptions, FsOptions, format_volume};
use ntfs::Ntfs;

#[derive()]
struct SPartition {
	pub name: String,
	pub ptype: PType,
	pub size: u64,
	pub flags: u64,
	pub id: Cell<Option<u32>>,
	pub startb: Cell<Option<u64>>,
	pub endb: Cell<Option<u64>>,
}
impl SPartition {
	pub fn fdisk<'a>(&self, file: &'a File) -> Result<StreamSlice<&'a File>> {
		let start = self
			.startb
			.get()
			.ok_or_else(|| anyhow!("Partition {} start unknown", self.name))?;
		let end = self
			.endb
			.get()
			.ok_or_else(|| anyhow!("Partition {} end not set", self.name))?;

		let slice = StreamSlice::new(file, start, end)?;
		Ok(slice)
	}
}

pub fn pack(dir: &Path, out: &Path) -> Result<(), Error> {
	if out.exists() {
		fs::remove_file(out)?;
	}
	let mut img_file = File::create(out)?;

	// create partitions based on https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/oem-deployment-of-windows-desktop-editions-sample-scripts?view=windows-11&preserve-view=true#createpartitions-uefitxt
	// efi partition
	//TODO: format as fat32
	let mut efip = SPartition {
		name: format!("efi"),
		ptype: partition_types::EFI,
		size: 300 * 1024 * 1024,
		flags: 0,
		id: Cell::new(None),
		startb: Cell::new(None),
		endb: Cell::new(None),
	};

	// 16MiB msr partition
	let msrp = SPartition {
		name: format!("msr"),
		ptype: partition_types::MICROSOFT_RESERVED,
		size: 16 * 1024 * 1024,
		flags: 0,
		id: Cell::new(None),
		startb: Cell::new(None),
		endb: Cell::new(None),
	};

	//primary partition
	// TODO: format as ntfs
	let primaryp = SPartition {
		name: format!("Windows"),
		ptype: partition_types::BASIC,
		size: 2 * 1024 * 1024 * 1024,
		flags: 0,
		id: Cell::new(None),
		startb: Cell::new(None),
		endb: Cell::new(None),
	};

	// recovery partition
	// TODO: format as ntfs
	let recoveryp = SPartition {
		name: format!("Recovery"),
		ptype: partition_types::BASIC,
		size: 300 * 1024 * 1024,
		flags: 0x8000000000000001,
		id: Cell::new(None),
		startb: Cell::new(None),
		endb: Cell::new(None),
	};

	let spartitions: Vec<&SPartition> = vec![&efip, &msrp, &primaryp, &recoveryp];

	let block_size = 512;
	let lb_size = LogicalBlockSize::Lb512;
	let part_align = 1 * 1024 * 1024;
	let est_size: u64 = spartitions.iter().map(|p| p.size).sum::<u64>()
		+ (spartitions.len() as u64 * part_align) * 2
		+ (2 * 1024 * 1024);

	img_file.set_len(est_size)?; // 4GiB
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
			None,
		)?;
		spart.id.set(Some(id));
	}

	// find partition ofsets
	let mut device: File;
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

	// todo: apply formatting etc based on https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/oem-deployment-of-windows-desktop-editions-sample-scripts?view=windows-11&preserve-view=true#applyimagebat

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
		let mut hellof = root.create_file("hello.txt")?;
		hellof.write_all(b"Hello World!")?;

		fdisk = efip.fdisk(&device)?;
	}

	// format and write primary
	{
		let mut fdisk = primaryp.fdisk(&device)?;
        let mut ntfs = Ntfs::new(&mut fdisk)?;
        let root = ntfs.root_directory(&mut fdisk)?;
	}

	Ok(())
}
