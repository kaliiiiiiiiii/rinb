use rdisk::{Disk, ReadAt, WriteAt, vhd::VhdImage};
use std::{
	fmt,
	io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write},
};
pub struct VhdStream<'a> {
	image: &'a VhdImage,
	pos: u64,
}

impl<'a> VhdStream<'a> {
	pub fn new(image: &'a VhdImage) -> Self {
		VhdStream { image, pos: 0 }
	}
}

impl<'a> Read for VhdStream<'a> {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
		let read_len = buf.len();
		let n = match self.image.read_at(self.pos, &mut buf[..read_len]) {
			Ok(n) => n,
			Err(rdisk::Error::ReadBeyondEOD) => 0, // treat as EOF
			Err(e) => return Err(Error::new(ErrorKind::Other, format!("{e:?}"))),
		};
		self.pos += n as u64;
		Ok(n)
	}
}

impl<'a> Write for VhdStream<'a> {
	fn write(&mut self, buf: &[u8]) -> Result<usize> {
		// Make sure we don't try to write past the end of the image

		let write_len = buf.len();
		let n = match self.image.write_at(self.pos, &buf[..write_len]) {
			Ok(n) => n,
			Err(rdisk::Error::WriteBeyondEOD) => 0,
			Err(e) => return Err(Error::new(ErrorKind::Other, format!("{e:?}"))),
		};
		self.pos += n as u64;
		Ok(n)
	}

	fn flush(&mut self) -> Result<()> {
		Ok(()) // no-op unless you want to flush underlying storage
	}
}

impl<'a> Seek for VhdStream<'a> {
	fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
		// shouldn't ever fail anyways
		let cap = self.image.capacity().unwrap();
		self.pos = match pos {
			SeekFrom::Start(n) => n,
			SeekFrom::End(n) => {
				let end = cap as i64 + n;
				if end < 0 {
					return Err(std::io::Error::new(
						std::io::ErrorKind::InvalidInput,
						"invalid seek before start",
					));
				}
				end as u64
			}
			SeekFrom::Current(n) => {
				let cur = self.pos as i64 + n;
				if cur < 0 {
					return Err(std::io::Error::new(
						std::io::ErrorKind::InvalidInput,
						"invalid seek before start",
					));
				}
				cur as u64
			}
		};
		Ok(self.pos)
	}
}

impl<'a> fmt::Debug for VhdStream<'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("VhdStream").field("pos", &self.pos).finish()
	}
}
