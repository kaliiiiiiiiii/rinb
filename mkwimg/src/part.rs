use std::{cell::Cell, fs::File};

use anyhow::{Result, anyhow};

use fscommon::StreamSlice;
use gpt::partition_types::Type as PType;

#[derive()]
pub struct SPartition {
	pub name: String,
	pub ptype: PType,
	pub size: u64,
	pub flags: u64,
	pub id: Cell<Option<u32>>,
	pub startb: Cell<Option<u64>>,
	pub endb: Cell<Option<u64>>,
	pub align: Option<u64>,
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
