use anyhow::Error;
use std::{env, fs, io, path::PathBuf};
use uuid::Uuid;
use wimlib::{
	Image,
	progress::{ProgressMsg, ProgressStatus},
	string::TStr,
	tstr,
};

use indicatif::{ProgressBar, ProgressStyle};

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
	pub fn tstr(self) -> Result<Box<TStr>, Box<dyn std::error::Error>> {
		let tstr = TStr::from_path(&self.path)?;
		Ok(tstr)
	}
}

impl Drop for TmpDir {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.path);
		let _ = fs::remove_dir(&self.path);
	}
}

pub trait ExpectEqual: Sized {
	fn expect_equal<M: AsRef<str>>(self, expected: Self, message: M) -> Result<Self, Error>;
}

impl<T: PartialEq + std::fmt::Debug> ExpectEqual for T {
	fn expect_equal<M: AsRef<str>>(self, expected: Self, message: M) -> Result<Self, Error> {
		if self != expected {
			panic!(
				"{}: expected {:?}, got {:?}",
				message.as_ref(),
				expected,
				self
			);
		}
		Ok(self)
	}
}

pub fn progress_callback<'b>(
	msg: &mut ProgressMsg<'b>,
	pb: &ProgressBar,
	bar_msg: String,
) -> ProgressStatus {
	match msg {
		ProgressMsg::WriteStreams {
			total_bytes,
			completed_bytes,
			..
		} => {
			pb.set_length(*total_bytes);
			pb.set_position(*completed_bytes);
			if (total_bytes == completed_bytes) {
				pb.finish_with_message(format!("finished {bar_msg}"));
			}
		}
		ProgressMsg::WriteMetadataBegin {} => {}
		ProgressMsg::WriteMetadataEnd {} => {}
		_ => {
			println!("Found unexpected progress message: {msg:?}");
		}
	}
	return ProgressStatus::Continue;
}

pub fn mk_pb(bar_msg: &String) -> ProgressBar {
	let pb = ProgressBar::new(1024 * 1024); // todo: Figure out a better estimate
	pb.set_style(
				ProgressStyle::default_bar()
					.template(
						"{msg} {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {binary_bytes}/{binary_total_bytes} ({eta}) {binary_bytes_per_sec}",
					)
					.unwrap()
					.progress_chars("#>-"),
			);
	pb.set_message(bar_msg.clone());
	return pb;
}

pub fn img_info<'a>(image: &'a Image<'a>) -> (&'a TStr, &'a TStr, &'a TStr) {
	let (name, descr, edition) = (
		image.property(tstr!("NAME")).unwrap(),
		image.property(tstr!("DESCRIPTION")).unwrap(),
		image
			.property(tstr!("WINDOWS/EDITIONID"))
			.unwrap_or(tstr!("")),
	);
	return (name, descr, edition);
}
