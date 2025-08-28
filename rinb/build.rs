use schemars::schema_for;
use serde_json::{Value, json};
use std::{collections::HashMap, env, fs, path::PathBuf};

mod config {
	include!("src/config.rs");
}

mod esd_downloader {
	include!("src/esd_downloader.rs");
}

use config::{Config, WinVer};
use esd_downloader::{FileInfo, WinEsdDownloader};

fn get_property(file: &FileInfo, prop: &str, version: &str) -> Option<String> {
	match prop {
		"version" => Some(version.to_string()),
		"architecture" => Some(file.architecture.clone()),
		"edition" => Some(file.edition.clone()),
		"lang" => Some(file.language_code.clone()),
		_ => None,
	}
}

fn build_conditions_recursive(
	files: &[&FileInfo],
	version: &str,
	props: &[&str],
	prefix: HashMap<&str, String>,
	out: &mut Vec<Value>,
) {
	if props.is_empty() {
		return;
	}

	let current_prop = props[0];
	let mut groups: HashMap<String, Vec<&FileInfo>> = HashMap::new();

	for f in files {
		if let Some(val) = get_property(f, current_prop, version) {
			groups.entry(val).or_default().push(*f);
		}
	}

	for (val, group) in groups {
		// collect unique next values
		if props.len() > 1 {
			let next_prop = props[1];
			let mut next_vals: Vec<String> = group
				.iter()
				.filter_map(|f| get_property(f, next_prop, version))
				.collect();
			next_vals.sort();
			next_vals.dedup();

			// build the "if" condition = all prefix props + current prop
			let mut if_props = serde_json::Map::new();
			for (k, v) in &prefix {
				if_props.insert((*k).to_string(), json!({ "const": v }));
			}
			if_props.insert(current_prop.to_string(), json!({ "const": val }));

			out.push(json!({
				"if": { "properties": if_props },
				"then": { "properties": { next_prop: { "enum": next_vals } } }
			}));

			// recurse deeper
			let mut new_prefix = prefix.clone();
			new_prefix.insert(current_prop, val.clone());
			build_conditions_recursive(&group, version, &props[1..], new_prefix, out);
		}
	}
}

pub fn build_dynamic_conditions(version_files: &HashMap<WinVer, Vec<FileInfo>>) -> Value {
	let mut all_of = Vec::new();

	// properties in order
	let props = vec!["version", "architecture", "edition", "lang"];

	for (ver, files) in version_files {
		let ver_str = ver.as_str();
		let files_ref: Vec<&FileInfo> = files.iter().collect();
		build_conditions_recursive(&files_ref, ver_str, &props, HashMap::new(), &mut all_of);
	}

	Value::Array(all_of)
}

fn main() -> anyhow::Result<()> {
	let mut schema = schema_for!(Config);
	let downloader = WinEsdDownloader::new("./.rinbcache/esd_cache")?;

	let files10: Vec<FileInfo> = downloader.files(&WinVer::Win10)?;
	let files11: Vec<FileInfo> = downloader.files(&WinVer::Win11)?;

	let mut version_files = HashMap::new();
	version_files.insert(WinVer::Win10, files10);
	version_files.insert(WinVer::Win11, files11);

	let all_of = build_dynamic_conditions(&version_files);
	schema.insert("allOf".to_owned(), all_of);

	let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let dest = manifest_dir.parent().unwrap().join("rinb_schema.json");

	fs::write(&dest, serde_json::to_string(&schema)?)?;

	println!("cargo:warning=Schema written to {}", dest.display());
	Ok(())
}
