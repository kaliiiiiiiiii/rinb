use anyhow::{Error, Result, anyhow};
use std::{env, fs::{self, File}, io, path::PathBuf};
use uuid::Uuid;

pub struct TmpDir {
	pub path: PathBuf,
}

impl TmpDir {
	pub fn new() -> Result<Self, io::Error> {
		let unique_name = format!("rinb_tmp_dirs_{}", Uuid::new_v4());
		let tmp_path = &env::temp_dir().join(unique_name);
		fs::create_dir(tmp_path)?;
		Ok(TmpDir {
			path: tmp_path.to_path_buf(),
		})
	}
}

impl Drop for TmpDir {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.path);
		let _ = fs::remove_dir(&self.path);
	}
}

pub trait ExpectEqual: Sized {
	fn expect_equal<M: AsRef<str>>(&self, expected: Self, message: M) -> Result<&Self, Error>;
}

impl<T: PartialEq + std::fmt::Debug> ExpectEqual for T {
	fn expect_equal<M: AsRef<str>>(&self, expected: Self, message: M) -> Result<&Self, Error> {
		if self != &expected {
			return Err(anyhow!(
				"{}: expected {:?}, got {:?}",
				message.as_ref(),
				expected,
				self
			));
		}
		Ok(self)
	}
}
