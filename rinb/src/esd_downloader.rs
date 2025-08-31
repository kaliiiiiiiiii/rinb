use crate::config::{Config, MajorWinVer};

use std::fs::{self, File};
use std::io::{self, BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use std::string::String;

use anyhow::{Error, Ok, Result, anyhow};
use roxmltree::Document;
use sha1::{Digest, Sha1};
use tempfile::NamedTempFile;

#[derive(Debug)]
pub struct FileInfo {
	pub file_name: String,
	pub language_code: String,
	pub language: String,
	pub edition: String,
	pub architecture: String,
	pub size: u64,
	pub sha1: String,
	pub file_path: String,
}

// Traversal function

fn find_files(xml: &str) -> Result<Vec<FileInfo>, Error> {
	let doc = Document::parse(xml)?;
	let mut result = Vec::new();

	// Traverse all <File> nodes
	for file_node in doc.descendants().filter(|n| n.has_tag_name("File")) {
		let get_text = |tag: &str| {
			file_node
				.children()
				.find(|c| c.has_tag_name(tag))
				.and_then(|c| c.text())
				.unwrap_or_default()
				.to_string()
		};

		let size = get_text("Size").parse::<u64>().unwrap_or(0);

		result.push(FileInfo {
			file_name: get_text("FileName"),
			language_code: get_text("LanguageCode"),
			language: get_text("Language"),
			edition: get_text("Edition"),
			architecture: get_text("Architecture"),
			size,
			sha1: get_text("Sha1"),
			file_path: get_text("FilePath"),
		});
	}

	Ok(result)
}

pub fn filename_without_extension(url: &String) -> Result<String, Error> {
	// Strip query parameters and fragment
	let end = url.find(|c| c == '?' || c == '#').unwrap_or(url.len());
	let path = &url[..end];

	// Get the last segment after '/'
	let filename = match path.rfind('/') {
		Some(pos) if pos + 1 < path.len() => &path[pos + 1..],
		_ => return Err(Error::msg(format!("No filename found in URL:{url}"))),
	};

	// Remove extension if any
	match filename.rfind('.') {
		Some(dot_pos) if dot_pos > 0 => Ok(filename[..dot_pos].to_string()),
		_ => Ok(filename.to_string()), // no extension found
	}
}

pub fn extract_cab_file(_data: &[u8], _filename: &str) -> Result<Vec<u8>, Error> {
	let cursor = Cursor::new(_data);
	let mut cabinet = cab::Cabinet::new(cursor)?;
	let mut reader = cabinet.read_file(_filename)?;
	let mut buffer = Vec::new();
	reader.read_to_end(&mut buffer)?;
	Ok(buffer)
}

pub struct WinEsdDownloader {
	cache_directory: PathBuf,
	http_client: reqwest::blocking::Client,
}

impl WinEsdDownloader {
	pub fn new(cache_directory: impl AsRef<Path>) -> Result<Self> {
		let cache_directory = cache_directory.as_ref().to_path_buf();
		fs::create_dir_all(&cache_directory)?;

		// Download and parse products.xml
		let client = reqwest::blocking::Client::new();

		Ok(Self {
			cache_directory,
			http_client: client,
		})
	}

	pub fn files(&self, win_ver: &MajorWinVer) -> Result<Vec<FileInfo>, Error> {
		let url = match win_ver {
			MajorWinVer::Win10 => "https://go.microsoft.com/fwlink/?LinkId=2156292",
			MajorWinVer::Win11 => "https://go.microsoft.com/fwlink/?LinkId=841361",
		};
		let response = self.http_client.get(url).send()?.bytes()?;

		let xml_bytes = extract_cab_file(&response, "products.xml")?;
		let xml_str = String::from_utf8(xml_bytes.clone())?;
		return Ok(find_files(&xml_str)?);
	}

	pub fn download_tmp(&self, config: &Config) -> Result<NamedTempFile> {
		let path = self.download(config)?;
		let mut tmp_file = NamedTempFile::new()?;

		let mut source_file = File::open(path)?;
		io::copy(&mut source_file, &mut tmp_file)?;

		Ok(tmp_file)
	}

	pub fn download(&self, config: &Config) -> Result<PathBuf, Error> {
		let (expected_size, expected_sha1, url): (u64, String, String);

		// figure out pinning, sha1, size, url etc.

		if let Some(cfgurl) = &config.url {
			url = cfgurl.clone();
			let (sha1, size) = config.parse_sha1size()?; // assuming parse_sha1size can borrow &self
			expected_sha1 = sha1;
			expected_size = size;
		} else {
			let file_info = self.find_file_info(&config)?;
			url = file_info.file_path;
			let sha1size = (file_info.sha1, file_info.size);

			if let Some(expected_sha1size) = config.parse_sha1size().ok() {
				assert_eq!(
					sha1size, expected_sha1size,
					"Mismatch between config.sha1size and actual file info reported by the endpoint"
				);
			}
			expected_sha1 = sha1size.0;
			expected_size = sha1size.1;
		}
		let file_name = filename_without_extension(&url)?;
		print!("Esd file from: {url}");

		let cache_file_name = format!(
			"{}-{}-{}-{}-{}.esd",
			file_name,
			config.lang,
			config.edition,
			config.arch.as_str(),
			expected_sha1
		);

		let cache_file_path = &self.cache_directory.join(cache_file_name);

		// Check if file exists and verify hash
		if cache_file_path.exists() {
			let existing_sha1 = self.calc_sha1(&cache_file_path)?;
			let existing_size = fs::metadata(cache_file_path)?.len();

			if existing_sha1.eq_ignore_ascii_case(&expected_sha1) && existing_size == expected_size
			{
				return Ok(cache_file_path.to_path_buf());
			}

			eprintln!(
				"Found existing modified or corrupted file: {cache_file_path:?}.\nGot SHA1: {existing_sha1}\nExpected:{expected_sha1}\nGot size:{existing_size}\nExpected:{expected_size}\n Deleting and downloading again.",
			);

			fs::remove_file(&cache_file_path)?;
		}

		// Download the file
		let mut response = self.http_client.get(url).send()?;
		let mut file = File::create(&cache_file_path)?;
		io::copy(&mut response, &mut file)?;

		// Verify downloaded file
		let actual_sha1 = self.calc_sha1(&cache_file_path)?;
		let existing_size = fs::metadata(cache_file_path)?.len();

		if !(actual_sha1.eq_ignore_ascii_case(&expected_sha1) && existing_size == expected_size) {
			fs::remove_file(&cache_file_path)?;
			return Err(anyhow!(
				"SHA-1 verification failed for {cache_file_path:?}.\nGot SHA1: {actual_sha1}\nExpected:{expected_sha1}\nGot size:{existing_size}\nExpected:{expected_size}\n Deleting and downloading again.",
			));
		}

		Ok(cache_file_path.to_path_buf())
	}

	fn find_file_info(&self, config: &Config) -> Result<FileInfo, Error> {
		let files = self.files(&config.version)?;

		let matching_files: Vec<FileInfo> = files
			.into_iter()
			.filter(|file| {
				file.language_code.eq_ignore_ascii_case(&config.lang)
					&& file.edition.eq_ignore_ascii_case(&config.edition)
					&& file.architecture.eq_ignore_ascii_case(config.arch.as_str())
			})
			.collect();

		let file = match matching_files.len() {
			0 => {
				return Err(anyhow!(
					"No matching file found for language: {}, edition: {}, architecture: {}, version:{}",
					config.lang,
					config.edition,
					config.arch.as_str(),
					config.version.as_str()
				));
			}
			1 => matching_files.into_iter().next().unwrap(), // exactly one match
			_ => {
				return Err(anyhow!(
					"Multiple matching files found for language: {}, edition: {}, architecture: {}, version:{}",
					config.lang,
					config.edition,
					config.arch.as_str(),
					config.version.as_str()
				));
			}
		};

		Ok(file)
	}

	fn calc_sha1(&self, file_path: &Path) -> Result<String> {
		let file = File::open(file_path)?;
		let mut reader = BufReader::with_capacity(65536, file); // 64 KB buffer
		let mut hasher = Sha1::new();
		let mut buffer = [0; 65536];

		loop {
			let bytes_read = reader.read(&mut buffer)?;
			if bytes_read == 0 {
				break;
			}
			hasher.update(&buffer[..bytes_read]);
		}

		Ok(hex::encode(hasher.finalize()))
	}
}
