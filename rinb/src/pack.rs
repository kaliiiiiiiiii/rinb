use anyhow::Result;
use std::path::Path;
use std::process::Command;

pub fn mkiso(isodir: &Path, outiso: &Path) -> Result<()> {
	#[cfg(windows)]
	{
		let output = Command::new("oscdimg")
			.args([
				"-LDevWin_ISO_windows",
				"-m",
				"-u2",
				"-h",
				&isodir.to_string_lossy(),
				&outiso.to_string_lossy(),
				"-pEF",
				&format!(
					"-bootdata:2#p0,e,b{0}/boot/etfsboot.com#pEF,e,b{0}/efi/microsoft/boot/efisys.bin",
					isodir.to_string_lossy()
				),
			])
			.output()?; // capture output

		if output.status.success() {
			println!("ISO created successfully!");
		} else {
			eprintln!("oscdimg failed!");
			let stderr = String::from_utf8_lossy(&output.stderr);
			eprintln!("Stderr:\n{}", stderr);
		}

		Ok(())
	}
	#[cfg(not(windows))]
	todo!()
}
