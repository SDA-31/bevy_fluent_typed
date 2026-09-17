//! Copy only the MIT library and example, never the consuming game's workspace.
use crate::{Options, Result};
use std::{
	fs,
	path::{Path, PathBuf},
	process::{Command, Output},
};

pub(crate) struct Fixture<'a> {
	pub(crate) path: PathBuf,
	options: &'a Options,
}

impl<'a> Fixture<'a> {
	pub(crate) fn new(source: &Path, options: &'a Options, version: &str) -> Result<Self> {
		// Retain failed fixtures and audit logs as well as successful ones.
		let path = tempfile::Builder::new()
			.prefix(&format!("bevy-fluent-{version}-"))
			.tempdir()?
			.keep();
		println!("Fixture: {}", path.display());

		for name in [
			"Cargo.toml",
			"LICENSE",
			"README.md",
			"GUIDE.md",
			".rustfmt.toml",
			"src",
			"docs",
			"codegen_bridge",
			"examples/minimal",
		] {
			copy(&source.join(name), &path.join(name))?;
		}

		let manifest = path.join("Cargo.toml");
		let generator = serde_json::to_string(&options.generator.to_string_lossy())?;
		fs::write(
			&manifest,
			format!(
				"{}\n[workspace]\nmembers = [\"codegen_bridge\", \"examples/minimal\", \"version-pins\"]\nresolver = \"3\"\n[patch.crates-io]\nfluent_typed_codegen = {{ path = {generator} }}\n[profile.dev]\ndebug = 0\n",
				fs::read_to_string(&manifest)?
			),
		)?;
		fs::create_dir(path.join("version-pins"))?;
		fs::write(
			path.join("version-pins/lib.rs"),
			"// Resolution constraints only.\n",
		)?;
		Ok(Self { path, options })
	}

	pub(crate) fn pin(&self, names: &[String], version: &str) -> Result<()> {
		let mut manifest = String::from(
			"[package]\nname = \"bevy-version-pins\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[lib]\npath = \"lib.rs\"\n[dependencies]\n",
		);

		for name in names {
			manifest.push_str(&format!(
				"{name} = {{ version = \"={version}\", default-features = false }}\n"
			));
		}

		fs::write(self.path.join("version-pins/Cargo.toml"), manifest)?;
		Ok(())
	}

	pub(crate) fn cargo(&self, arguments: &[&str], capture: bool) -> Result<Output> {
		println!("+ cargo {}", arguments.join(" "));
		let mut command = Command::new("cargo");
		command.current_dir(&self.path).args(arguments);

		if self.options.offline {
			command.arg("--offline");
		}

		if let Some(target) = &self.options.target {
			command.env("CARGO_TARGET_DIR", target);
		}

		if !capture {
			command
				.stdout(std::process::Stdio::inherit())
				.stderr(std::process::Stdio::inherit());
		}

		Ok(command.output()?)
	}

	pub(crate) fn success(&self, arguments: &[&str]) -> Result<()> {
		let result = self.cargo(arguments, false)?;

		if !result.status.success() {
			return Err(format!("cargo {} failed: {}", arguments.join(" "), result.status).into());
		}

		Ok(())
	}
}

fn copy(source: &Path, destination: &Path) -> Result<()> {
	let metadata = source.symlink_metadata()?;

	if metadata.file_type().is_symlink() {
		return Err(format!("refusing to follow fixture symlink: {}", source.display()).into());
	}

	if !metadata.is_dir() {
		fs::copy(source, destination)?;
		return Ok(());
	}

	fs::create_dir_all(destination)?;

	for entry in fs::read_dir(source)? {
		let entry = entry?;

		if matches!(
			entry.file_name().to_str(),
			Some("target" | "Cargo.lock" | ".git")
		) {
			continue;
		}

		copy(&entry.path(), &destination.join(entry.file_name()))?;
	}

	Ok(())
}
