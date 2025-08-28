use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn default_lang() -> String {
	"en-US".to_string()
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
	Amd64,
	Arm64,
}

impl Arch {
	pub fn as_str(&self) -> &'static str {
		match self {
			Arch::Amd64 => "amd64",
			Arch::Arm64 => "arm6",
		}
	}
}

fn default_arch() -> Arch {
	Arch::Amd64
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WinVer {
	#[serde(rename="10")]
	Win10,
	#[serde(rename="11")]
	Win11,
}

fn default_winver() -> WinVer {
	WinVer::Win11
}

fn default_edition() -> String {
	"Professional".to_string()
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Config {
	#[doc = "Lang code, e.g. en-US"]
	#[serde(default = "default_lang")]
	pub lang: String,
	#[serde(default = "default_arch")]
	pub arch: Arch,
	#[serde(default = "default_edition")]
	pub editon: String,
	#[serde(default = "default_winver")]
	pub version: WinVer,
}
