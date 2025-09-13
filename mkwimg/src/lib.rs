use anyhow::{Error, Result, anyhow};
use std::{
	cell::Cell,
	fs::{self, File},
	io::{Read, Write},
	path::{Path, PathBuf},
};

use indicatif::{ProgressBar, ProgressStyle};

use fscommon::StreamSlice;

use gpt::{
	GptConfig,
	disk::LogicalBlockSize,
	mbr::ProtectiveMBR,
	partition::Partition,
	partition_types::{self, Type as PType},
};
use rdisk::vhd::{VhdImage, VhdKind};

use fatfs::{
	Dir, FatType, FileSystem, FormatVolumeOptions, FsOptions, ReadWriteSeek, format_volume,
};

mod utils;
use utils::VhdStream;

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

pub fn dir2fatsize(path: impl AsRef<Path>) -> Result<u64> {
	let cluster_size = 32 * 1024; // https://github.com/rafalh/rust-fatfs/blob/4eccb50d011146fbed20e133d33b22f3c27292e7/src/boot_sector.rs#L490

	let mut total_size = 0;
	let mut stack = vec![path.as_ref().to_path_buf()];

	while let Some(current_path) = stack.pop() {
		let entries = fs::read_dir(&current_path)?;

		for entry in entries {
			let entry = entry?;

			let meta = entry.metadata()?;

			let name_len = entry.file_name().to_string_lossy().len();
			let lfn_entries = (name_len + 12) / 13; // FAT32 long name entries
			let dir_entry_size = 32 + (lfn_entries as u64 * 32);

			if meta.is_dir() {
				// Add directory overhead (like "." and "..")
				total_size += 32 + dir_entry_size;
				stack.push(entry.path());
			} else {
				let file_size = meta.len();
				let clusters = (file_size + cluster_size - 1) / cluster_size;
				total_size += clusters * cluster_size + dir_entry_size;
			}
		}
	}

	Ok(total_size)
}

fn dir2fat<'a, T, P>(fs_dir: &Dir<'a, T>, src_path: P) -> Result<(), Error>
where
	T: ReadWriteSeek + 'a,
	P: AsRef<Path>,
{
	let src_path = src_path.as_ref();

	// Collect all files first to get total size for progress bar
	let mut total_bytes = 0;
	let mut stack: Vec<PathBuf> = vec![src_path.to_path_buf()];

	while let Some(path) = stack.pop() {
		if path.is_dir() {
			for entry in fs::read_dir(&path)? {
				let entry = entry?;
				stack.push(entry.path());
			}
		} else if path.is_file() {
			let size = path.metadata().map(|m| m.len()).unwrap_or(0);
			total_bytes += size;
		}
	}

	// Initialize progress bar
	let pb = ProgressBar::new(total_bytes);
	pb.set_message("Writing dir to FAT32");
	pb.set_style(
    ProgressStyle::default_bar()
        .template(
            "{msg} {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {binary_bytes}/{binary_total_bytes} ({eta}) {binary_bytes_per_sec}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

	// Reset stack for actual copy
	let mut stack: Vec<(PathBuf, Dir<'a, T>)> = vec![];

	for entry in fs::read_dir(src_path)? {
		let entry = entry?;
		stack.push((entry.path(), fs_dir.clone()));
	}

	while let Some((path, parent_dir)) = stack.pop() {
		if path.is_dir() {
			let dir_name = path.file_name().unwrap().to_str().unwrap();
			let new_dir = parent_dir.create_dir(dir_name)?;

			for entry in fs::read_dir(&path)? {
				let entry = entry?;
				stack.push((entry.path(), new_dir.clone()));
			}
		} else if path.is_file() {
			let file_name = path.file_name().unwrap().to_str().unwrap();
			let mut src_file = fs::File::open(&path)?;
			let mut fs_file = parent_dir.create_file(file_name)?;

			let mut buffer = vec![0u8; 4 * 1024 * 1024]; // 4 MB buffer
			loop {
				let n = src_file.read(&mut buffer)?;
				if n == 0 {
					break;
				}
				fs_file.write_all(&buffer[..n])?;
				pb.inc(n as u64);
			}
		}
	}

	pb.finish_with_message("Writing dir to FAT");
	Ok(())
}

pub fn pack(dir: &Path, out: &Path) -> Result<(), Error> {
	if out.exists() {
		fs::remove_file(out)?;
	}
	// let mut img_file = File::create(out)?;

	// create partitions based on https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/oem-deployment-of-windows-desktop-editions-sample-scripts?view=windows-11&preserve-view=true#createpartitions-uefitxt
	// efi partition
	//TODO: format as fat32
	let efip = SPartition {
		name: format!("efi"),
		ptype: partition_types::EFI,
		size: dir2fatsize(dir)?,
		flags: 0,
		id: Cell::new(None),
		startb: Cell::new(None),
		endb: Cell::new(None),
	};

	let spartitions: Vec<&SPartition> = vec![&efip];

	let block_size = 512;
	let lb_size = LogicalBlockSize::Lb512;
	let part_align = 1 * 1024 * 1024;
	let est_size: u64 = spartitions.iter().map(|p| p.size).sum::<u64>()
		+ (spartitions.len() as u64 * part_align) * 2
		+ (2 * 1024 * 1024);

	// vhd + 1MiB buffer
	let vhd_img = VhdImage::create_fixed(out.to_string_lossy(), est_size+1024 * 1024)
		.map_err(|e| anyhow::anyhow!(e))?;
	let mut img_file = VhdStream::new(&vhd_img);
	// img_file.set_len(est_size)?; // 4GiB

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

		// fails with ;01HBdsDxe: failed to load Boot0001 "UEFI VBOX HARDDISK VB9a76c119-66b61545 " from PciRoot(0x0)/Pci(0xD,0x0)/Sata(0x0,0xFFFF,0x0): Not Found
		dir2fat(&root, dir)?;
	}
	Ok(())
}
