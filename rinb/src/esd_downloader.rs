use std::fs::{self, File};
use std::io::{self, Cursor, Read, BufReader};
use std::path::{Path, PathBuf};
use std::string::String;

use anyhow::{Result, anyhow};
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
fn find_files(xml: &str) -> Vec<FileInfo> {
    let doc = Document::parse(xml).unwrap();
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
            size: size,
            sha1: get_text("Sha1"),
            file_path: get_text("FilePath"),
        });
    }

    result
}

pub fn extract_cab_file(
    _data: &[u8],
    _filename: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let cursor = Cursor::new(_data);
    let mut cabinet = cab::Cabinet::new(cursor)?;
    let mut reader = cabinet.read_file(_filename)?;
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    Ok(buffer)
}

pub struct WinEsdDownloader {
    cache_directory: PathBuf,
    files: Vec<FileInfo>,
    http_client: reqwest::blocking::Client,
}

impl WinEsdDownloader {
    pub fn new(cache_directory: impl AsRef<Path>) -> Result<Self> {
        let cache_directory = cache_directory.as_ref().to_path_buf();
        fs::create_dir_all(&cache_directory)?;

        // Download and parse products.xml
        let client = reqwest::blocking::Client::new();
        let response = client
            .get("https://go.microsoft.com/fwlink/?LinkId=2156292")
            .send()?
            .bytes()?;

        let xml_bytes = extract_cab_file(&response, "products.xml").unwrap();
        let xml_str = String::from_utf8(xml_bytes.clone())?;
        let files = find_files(&xml_str);

        Ok(Self {
            cache_directory,
            files: files,
            http_client: client,
        })
    }

    pub fn download_tmp(
        &self,
        language: &str,
        edition: &str,
        architecture: &str,
    ) -> Result<NamedTempFile> {
        let path = self.download(language, edition, architecture)?;
        let mut tmp_file = NamedTempFile::new()?;

        let mut source_file = File::open(path)?;
        io::copy(&mut source_file, &mut tmp_file)?;

        Ok(tmp_file)
    }

    pub fn download(&self, language: &str, edition: &str, architecture: &str) -> Result<PathBuf> {
        let file_info = self.find_file_info(language, edition, architecture)?;

        let normalized_arch = if architecture.eq_ignore_ascii_case("amd64") {
            "x64"
        } else {
            architecture
        };

        // Generate cache file name: {original_name}-{language}-{edition}-{architecture}-{sha1}.esd
        let file_stem = Path::new(&file_info.file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let cache_file_name = format!(
            "{}-{}-{}-{}-{}.esd",
            file_stem, language, edition, normalized_arch, file_info.sha1
        );

        let cache_file_path = &self.cache_directory.join(cache_file_name);

        // Check if file exists and verify hash
        if cache_file_path.exists() {
            let existing_sha1 = self.calc_sha1(&cache_file_path)?;
            let existing_size = fs::metadata(cache_file_path)?.len();

            if existing_sha1.eq_ignore_ascii_case(&file_info.sha1)
                && existing_size == file_info.size
            {
                return Ok(cache_file_path.to_path_buf());
            }

            eprintln!(
                "Found existing modified or corrupted file: {}.\nGot SHA1: {}\nExpected:{}\nGot size:{}\nExpected:{}\n Deleting and downloading again.",
                cache_file_path.display(),
                file_info.sha1,
                existing_sha1,
                existing_size,
                file_info.size
            );

            fs::remove_file(&cache_file_path)?;
        }

        // Download the file
        let mut response = self.http_client.get(&file_info.file_path).send()?;
        let mut file = File::create(&cache_file_path)?;
        io::copy(&mut response, &mut file)?;

        // Verify downloaded file
        let actual_sha1 = self.calc_sha1(&cache_file_path)?;
        let existing_size = fs::metadata(cache_file_path)?.len();

        if !(actual_sha1.eq_ignore_ascii_case(&file_info.sha1) && existing_size == file_info.size) {
            fs::remove_file(&cache_file_path)?;
            return Err(anyhow!(
                "SHA-1 verification failed for {}.\nGot SHA1: {}\nExpected:{}\nGot size:{}\nExpected:{}\n Deleting and downloading again.",
                cache_file_path.display(),
                file_info.sha1,
                actual_sha1,
                existing_size,
                file_info.size
            ));
        }

        Ok(cache_file_path.to_path_buf())
    }

    fn find_file_info(
        &self,
        language: &str,
        edition: &str,
        architecture: &str,
    ) -> Result<&FileInfo> {
        let normalized_arch = if architecture.eq_ignore_ascii_case("amd64") {
            "x64"
        } else {
            architecture
        };

        self.files
            .iter()
            .find(|file| {
                file.language_code.eq_ignore_ascii_case(language)
                    && file.edition.eq_ignore_ascii_case(edition)
                    && file.architecture.eq_ignore_ascii_case(normalized_arch)
            })
            .ok_or_else(|| {
                anyhow!(
                    "No matching file found for language: {}, edition: {}, architecture: {}",
                    language,
                    edition,
                    architecture
                )
            })
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
