use schemars::schema_for;
use serde_json::{Map, Value, json};
use std::{collections::HashMap, env, fs, path::PathBuf};

mod config {
	include!("src/config.rs");
}

mod esd_downloader {
	include!("src/esd_downloader.rs");
}

mod download {
	include!("src/download.rs");
}

mod utils {
	include!("src/utils.rs");
}


use config::{Config, MajorWinVer};
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

fn build_property_enums(version_files: &HashMap<MajorWinVer, Vec<FileInfo>>) -> Map<String, Value> {
	let props = vec![
		// "version", "architecture", // we don't need these dynamically
		"edition", "lang",
	];
	let mut defs = Map::new();

	for &prop in &props {
		let mut vals: Vec<String> = Vec::new();

		for (ver, files) in version_files {
			let ver_str = ver.as_str();
			for f in files {
				if let Some(val) = get_property(f, prop, ver_str) {
					vals.push(val);
				}
			}
		}

		vals.sort();
		vals.dedup();

		if !vals.is_empty() {
			defs.insert(prop.to_string(), json!({ "enum": vals }));
		}
	}

	defs
}

fn main() -> anyhow::Result<()> {
	let mut schema = schema_for!(Config);
	let downloader = WinEsdDownloader::new("./.rinbcache/esd_cache")?;

	let files10: Vec<FileInfo> = downloader.files(&MajorWinVer::Win10)?;
	let files11: Vec<FileInfo> = downloader.files(&MajorWinVer::Win11)?;

	let mut version_files = HashMap::new();
	version_files.insert(MajorWinVer::Win10, files10);
	version_files.insert(MajorWinVer::Win11, files11);

	let enums = build_property_enums(&version_files);

	// modify schema
	let schema_obj = schema.as_object_mut().unwrap();

	let props = schema_obj
		.entry("properties".to_string())
		.or_insert_with(|| Value::Object(Map::new()))
		.as_object_mut()
		.unwrap();

	// iterate over enums and collect inserts for defs
	let mut defs_inserts = Vec::new();
	for (k, v) in enums.iter() {
		defs_inserts.push((k.clone(), v.clone()));
		let prop = props.get_mut(k).unwrap().as_object_mut().unwrap();
		prop.remove("type");
		prop.insert("$ref".to_string(), json!(format!("#/$defs/{k}")));
	}

	// insert defs after modifying props
	let defs = schema_obj
		.entry("$defs".to_string())
		.or_insert_with(|| Value::Object(Map::new()))
		.as_object_mut()
		.unwrap();

	for (k, v) in defs_inserts {
		defs.insert(k, v);
	}
    
	// require sha1size if url is specified
	schema_obj.insert(
		"allOf".to_string(),
		json!([
			{
				"if": {
					"not": {
						"properties": {
							"url": { "const": null }
						}
					}
				},
				"then": {
					"required": ["sha1size"],
					"properties": {
						"sha1size": {
							"type": "string",
							"pattern": "^[0-9a-f]{40}:[0-9]+$"
						}
					}
				}
			}
		]),
	);

	let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let dest = manifest_dir.parent().unwrap().join("rinb_schema.json");

	fs::write(&dest, serde_json::to_string_pretty(&schema)?)?;

	println!("cargo:warning=Schema written to {}", dest.display());
	Ok(())
}
