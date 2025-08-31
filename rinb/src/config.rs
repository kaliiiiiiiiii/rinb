use anyhow::Error;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn default_lang() -> String {
	"en-us".to_string()
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Hash, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
	Amd64,
	Arm64,
	X86,
}

impl Arch {
	pub fn as_str(&self) -> &'static str {
		match self {
			Arch::Amd64 => "x64",
			Arch::Arm64 => "arm64",
			Arch::X86 => "x86",
		}
	}
}

fn default_arch() -> Arch {
	Arch::Amd64
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Hash, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum MajorWinVer {
	#[serde(rename = "10")]
	Win10,
	#[serde(rename = "11")]
	Win11,
}

impl MajorWinVer {
	pub fn as_str(&self) -> &'static str {
		match self {
			MajorWinVer::Win10 => "10",
			MajorWinVer::Win11 => "11",
		}
	}
}

fn default_major_winver() -> MajorWinVer {
	MajorWinVer::Win11
}

fn default_edition() -> String {
	"Professional".to_string()
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Clone)]
pub struct Config {
	#[schemars(default = "default_lang", description = "Target language")]
	pub lang: String,
	#[schemars(default = "default_arch", description = "Target architecture")]
	pub arch: Arch,
	#[schemars(default = "default_edition", description = "Windows edition")]
	pub edition: String,
	#[schemars(
		default = "default_major_winver",
		description = "Major windows version"
	)]
	pub version: MajorWinVer,
	#[schemars(
		regex(pattern = r"^[0-9a-f]{40}:[0-9]+$"),
		description = "{sha1}:{sizeInBytes} for pinning"
	)]
	pub sha1size: Option<String>,
	#[schemars(url, description = "Optional URL for pinning. Requires sha1size to be defined.")]
	pub url: Option<String>,
}

impl Config {
	pub fn parse_sha1size(&self) -> Result<(String, u64), Error> {
		if let Some(sha1sizestr) = &self.sha1size {
			let (sha1str, size_str) = sha1sizestr
				.split_once(':')
				.ok_or_else(|| Error::msg("sha1size must be in format '{sha1}:{sizeInBytes}'"))?;

			let size = size_str
				.parse::<u64>()
				.map_err(|_| Error::msg("size must be a valid u64"))?;

			return Ok((sha1str.to_string(), size))
		};
		Err(Error::msg("sha1size not provided"))
	}
}
