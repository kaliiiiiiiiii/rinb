use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn default_lang() -> String {
	"en-US".to_string()
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Hash, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
	Amd64,
	Arm64,
	X86
}

impl Arch {
	pub fn as_str(&self) -> &'static str {
		match self {
			Arch::Amd64 => "x64",
			Arch::Arm64 => "arm64",
			Arch::X86 => "x86"
		}
	}
}

fn default_arch() -> Arch {
	Arch::Amd64
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Hash, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum WinVer {
	#[serde(rename="10")]
	Win10,
	#[serde(rename="11")]
	Win11,
}

impl WinVer {
    pub fn as_str(&self) -> &'static str {
        match self {
            WinVer::Win10 => "10",
            WinVer::Win11 => "11",
        }
    }
}

fn default_winver() -> WinVer {
	WinVer::Win11
}

fn default_edition() -> String {
	"Professional".to_string()
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Config {
	#[serde(default = "default_lang")]
	pub lang: String,
	#[serde(default = "default_arch")]
	pub arch: Arch,
	#[serde(default = "default_edition")]
	pub editon: String,
	#[serde(default = "default_winver")]
	pub version: WinVer,
}
