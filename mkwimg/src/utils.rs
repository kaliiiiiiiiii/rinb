use std::{
	fmt, fs,
	io::{Error as IoError, ErrorKind, Read, Result as IoResult, Seek, SeekFrom, Write},
	path::{Path, PathBuf},
};

use anyhow::{Error, Result};
use indicatif::{ProgressBar, ProgressStyle};

use fatfs::{Dir, ReadWriteSeek};
use rdisk::{Disk, ReadAt, WriteAt, vhd::VhdImage};

pub struct VhdStream {
	image: VhdImage,
	pos: u64,
}

impl VhdStream {
	pub fn new(image: VhdImage) -> Self {
		VhdStream { image, pos: 0 }
	}
}

impl Read for VhdStream {
	fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
		let read_len = buf.len();
		let n = match self.image.read_at(self.pos, &mut buf[..read_len]) {
			Ok(n) => n,
			Err(rdisk::Error::ReadBeyondEOD) => 0, // treat as EOF
			Err(e) => return Err(IoError::new(ErrorKind::Other, format!("{e:?}"))),
		};
		self.pos += n as u64;
		Ok(n)
	}
}

impl Write for VhdStream {
	fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
		// Make sure we don't try to write past the end of the image

		let write_len = buf.len();
		let n = match self.image.write_at(self.pos, &buf[..write_len]) {
			Ok(n) => n,
			Err(rdisk::Error::WriteBeyondEOD) => 0,
			Err(e) => return Err(IoError::new(ErrorKind::Other, format!("{e:?}"))),
		};
		self.pos += n as u64;
		Ok(n)
	}

	fn flush(&mut self) -> IoResult<()> {
		Ok(()) // no-op unless you want to flush underlying storage
	}
}

impl Seek for VhdStream {
	fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
		// shouldn't ever fail anyways
		let cap = self.image.capacity().unwrap();
		self.pos = match pos {
			SeekFrom::Start(n) => n,
			SeekFrom::End(n) => {
				let end = cap as i64 + n;
				if end < 0 {
					return Err(IoError::new(
						ErrorKind::InvalidInput,
						"invalid seek before start",
					));
				}
				end as u64
			}
			SeekFrom::Current(n) => {
				let cur = self.pos as i64 + n;
				if cur < 0 {
					return Err(IoError::new(
						ErrorKind::InvalidInput,
						"invalid seek before start",
					));
				}
				cur as u64
			}
		};
		Ok(self.pos)
	}
}

impl fmt::Debug for VhdStream {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("VhdStream").field("pos", &self.pos).finish()
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

pub fn dir2fat<'a, T, P>(fs_dir: &Dir<'a, T>, src_path: P) -> Result<(), Error>
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
