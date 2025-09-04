use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;

use anyhow::{Error, Result, anyhow};
use indicatif::{ProgressBar, ProgressStyle};
use sha1::{Digest, Sha1};

pub fn fdownload<R: Read>(
	mut reader: R,
	cache_file_path: Option<&PathBuf>,
	expected_size: &u64,
	expected_sha1: &str,
	label: &str,
) -> Result<(), Error> {
	let cache_file: Option<File>;
	if let Some(filepath) = cache_file_path {
		cache_file = Some(File::create(filepath)?);
	} else {
		cache_file = Option::None;
	}

	// Setup progress bar
	let pb = ProgressBar::new(*expected_size);
	pb.set_style(
    ProgressStyle::default_bar()
        .template(
            "{msg} {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {binary_bytes}/{binary_total_bytes} ({eta}) {binary_bytes_per_sec}",
        )
        .unwrap()
        .progress_chars("#>-"),
);
	pb.set_message(label.to_string());

	let mut hasher = Sha1::new();
	let mut buffer = vec![0; 256 * 1024]; // 1MB buffer
	let mut writer = cache_file.as_ref().map(|f| BufWriter::new(f));

	let mut total_bytes = 0;
	let mut update_counter = 0;

	// do the actual processing
	loop {
		let bytes_read = reader.read(&mut buffer)?;
		if bytes_read == 0 {
			break;
		}

		if let Some(writer) = writer.as_mut() {
			writer.write_all(&buffer[..bytes_read])?;
		}
		hasher.update(&buffer[..bytes_read]);

		total_bytes += bytes_read as u64;
		update_counter += bytes_read;

		// Update progress every 16MB to reduce overhead
		if update_counter >= 16 * 1024 * 1024 {
			pb.set_position(total_bytes);
			update_counter = 0;
		}
	}

	// Flush any buffered writes
	if let Some(mut writer) = writer {
		writer.flush()?;
	}
	pb.set_position(total_bytes); // Final update

	let finish_message = format!("Finished {}", label);
	pb.finish_with_message(finish_message);

	// Verify SHA1
	let actual_sha1 = hex::encode(hasher.finalize());
	if actual_sha1 != expected_sha1 {
		return Err(anyhow!(
			"SHA1 mismatch: expected {}, got {}",
			expected_sha1,
			actual_sha1
		));
	}

	Ok(())
}

pub fn download_from_url(
	url: &String,
	cache_file_path: &PathBuf,
	expected_size: &u64,
	expected_sha1: &str,
) -> Result<(), Error> {
	let response = reqwest::blocking::get(url)?;
	let reader = response.error_for_status()?;

	fdownload(
		reader.take(*expected_size),
		Some(cache_file_path),
		expected_size,
		expected_sha1,
		&format!(
			"Downloading and verifying (hashing) {:?}\n",
			&cache_file_path.file_name().unwrap()
		),
	)?;

	Ok(())
}
