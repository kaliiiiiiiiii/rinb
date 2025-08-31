use anyhow::{Context, Error, Result};
use std::{fs, num::NonZero, path::PathBuf};

use hadris_iso::{
	BootEntryOptions, BootOptions, BootSectionOptions, EmulationType, FileInput, FileInterchange,
	FormatOption, IsoImage, PartitionOptions, PlatformId, Strictness,
};

fn add_files_from_dir(
	base: &PathBuf,
	files: &mut hadris_iso::FileInput,
	max_len: usize,
) -> Result<(), Error> {
	let mut stack = vec![base.to_path_buf()];

	while let Some(dir) = stack.pop() {
		for entry in fs::read_dir(&dir).context(format!("Failed to read directory: {dir:?}"))? {
			let entry = entry?;
			let path = entry.path();

			if path.is_dir() {
				stack.push(path);
			} else {
				let rel_path = path
					.strip_prefix(base)
					.context(format!(
						"Failed to compute relative path for {path:?} on {base:?}"
					))?
					.to_string_lossy()
					.to_string()
					.replace("\\", "/");
				if path
					.file_name()
					.unwrap()
					.to_string_lossy()
					.to_string()
					.len() > max_len
				{
					println!("{rel_path}")
				} else {
					files.append(hadris_iso::File {
						path: rel_path,
						data: hadris_iso::FileData::File(path.clone()),
					});
				}
			}
		}
	}

	Ok(())
}

pub fn pack(base_path: &PathBuf, outpath: &PathBuf) -> Result<(), Error> {
	// workaround https://github.com/hxyulin/hadris/blob/9159df0fdeab02cedaabc2bdda89f8153f2a5d75/crates/hadris-iso/src/lib.rs#L218
	fs::copy(
		base_path.join("boot/etfsboot.com"),
		base_path.join("etfsboot.com"),
	)?;
	fs::copy(
		base_path.join("efi/microsoft/boot/efisys.bin"),
		base_path.join("efisys.bin"),
	)?;

	let mut files = FileInput::empty();
	add_files_from_dir(base_path, &mut files, 32)?;

	let fmt_opts = FormatOption::default()
		.with_files(files)
		.with_level(FileInterchange::NonConformant)
		.with_boot_options(BootOptions {
			write_boot_catalogue: true,
			default: BootEntryOptions {
				boot_image_path: format!("etfsboot.com"),
				load_size: NonZero::new(4),
				emulation: EmulationType::NoEmulation,
				boot_info_table: false,
				grub2_boot_info: false,
			},
			entries: vec![(
				BootSectionOptions {
					platform_id: PlatformId::UEFI,
				},
				BootEntryOptions {
					boot_image_path: format!("efisys.bin"),
					load_size: None, // This means the size will be calculated
					emulation: EmulationType::NoEmulation,
					boot_info_table: false,
					grub2_boot_info: false,
				},
			)],
		})
		.with_volume_name(format!("RINB"))
		.with_format_options(PartitionOptions::GPT|PartitionOptions::PROTECTIVE_MBR)
		.with_strictness(Strictness::default());
	fs::create_dir_all(outpath.parent().unwrap())?;
	IsoImage::format_file(outpath, fmt_opts)?;
	Ok(())
}
