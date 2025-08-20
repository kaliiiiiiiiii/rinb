use clap::builder::Str;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;


fn default_lang() -> String {
    "en-US".to_string()
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    X64,
    Arm64,
}

impl Default for Arch {
    fn default() -> Self {
        Arch::X64
    }
}

fn default_arch() -> Arch {
    Arch::default()
}

fn default_edition() -> String {
    "Professional".to_string()
}


#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Config {
    #[doc = "Lang code, e.g. en-US"]
    #[serde(default="default_lang")]
    pub lang: String,
    #[serde(default="default_arch")]
    pub arch: Arch,
    #[serde(default="default_arch")]
    pub editon: String
}