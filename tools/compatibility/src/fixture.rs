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
			"examples/minimal",
			"examples/codegen",
			"examples/no_codegen",
			"examples/icu",
			"examples/asset_source",
		] {
			copy(&source.join(name), &path.join(name))?;
		}

		let backend = crate::matrix::backend(version)?;
		let bsn = matches!(backend.as_str(), "bevy-0-19" | "bevy-0-20");
		let mut generated_examples = vec!["codegen", "minimal", "icu", "asset_source"];

		if bsn {
			copy(&source.join("examples/bsn"), &path.join("examples/bsn"))?;
			generated_examples.push("bsn");
		}

		// Keep the smallest example free of backend-forwarding features: forwarding
		// a feature to the same dependency name would also enable it on the host.
		for example in generated_examples {
			let example_manifest = path.join(format!("examples/{example}/Cargo.toml"));
			let original = fs::read_to_string(&example_manifest)?;
			let features = if example == "minimal" {
				"\"watch\", \"codegen\""
			} else {
				"\"codegen\""
			};
			let dependency = format!("features = [{features}]");

			if !original.contains(&dependency) {
				return Err(format!("{example} example's runtime dependency changed").into());
			}

			fs::write(
				&example_manifest,
				original.replace(
					&dependency,
					&format!("default-features = false, features = [{features}, \"{backend}\"]"),
				),
			)?;
		}

		if !bsn {
			// Older engines use application-owned requests and native conditions.
			// Adapt copies only; public examples exercise discovery on 0.19/0.20.
			for (file, function, catalog, scope) in [
				(
					"examples/minimal/src/bin/typed_resources.rs",
					"show_hud",
					"texts::Translations",
					"texts::Translations",
				),
				(
					"examples/asset_source/src/main.rs",
					"show_title",
					"texts::Translations",
					"texts::ui::Hud",
				),
			] {
				adapt_startup(&path.join(file), function, catalog, scope)?;
			}

			for (file, scope, request) in [
				(
					"examples/codegen/tests/bytes.rs",
					"texts::ui::Greeting",
					true,
				),
				(
					"examples/minimal/tests/catalog/macros.rs",
					"Interface",
					false,
				),
			] {
				adapt_update(&path.join(file), scope, request)?;
			}
		}

		if bsn {
			let manifest = path.join("examples/bsn/Cargo.toml");
			let original = fs::read_to_string(&manifest)?;
			let scene_dependency = "bevy = { version = \"0.20.0\"";

			if !original.contains(scene_dependency) {
				return Err("BSN example's scene dependency changed".into());
			}

			fs::write(
				manifest,
				original.replace(
					scene_dependency,
					&format!("bevy = {{ version = \"{version}\""),
				),
			)?;

			if backend == "bevy-0-19" {
				let tests = path.join("examples/bsn/src/tests.rs");
				fs::write(
					&tests,
					fs::read_to_string(&tests)?
						.replace("native/bevy_0_20.rs", "native/bevy_0_19.rs"),
				)?;
			}
		}

		let bsn_member = if bsn { ", \"examples/bsn\"" } else { "" };

		let manifest = path.join("Cargo.toml");
		let generator = serde_json::to_string(&options.generator.to_string_lossy())?;
		fs::write(
			&manifest,
			format!(
				"{}\n[workspace]\nmembers = [\"examples/minimal\", \"examples/codegen\", \"examples/no_codegen\", \"examples/icu\", \"examples/asset_source\", \"version-pins\"{bsn_member}]\nresolver = \"3\"\n[patch.crates-io]\nfluent_typed_codegen = {{ path = {generator} }}\n[profile.dev]\ndebug = 0\n",
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
			let version = crate::matrix::package_version(name, version);
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
		command.current_dir(&self.path);

		if self.options.offline {
			command.arg("--offline");
		}

		command.args(arguments);

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

fn adapt_startup(path: &Path, function: &str, catalog: &str, scope: &str) -> Result<()> {
	let source = fs::read_to_string(path)?;
	let native = format!(".add_systems(Startup, {function})");

	if source.matches(&native).count() != 1 {
		return Err(format!("native registration changed in {}", path.display()).into());
	}

	let replacement = format!(
		".add_systems(Startup, |mut localization: ResMut<bevy_fluent_typed::Localization<{catalog}>>| localization.load::<{scope}>())\n.add_systems(Update, {function}.run_if(resource_exists::<{scope}>.and(run_once)))"
	);
	fs::write(path, source.replace(&native, &replacement))?;
	Ok(())
}

fn adapt_update(path: &Path, scope: &str, request: bool) -> Result<()> {
	let source = fs::read_to_string(path)?;
	let native = ".add_systems(Update, move |";

	if source.matches(native).count() != 1 {
		return Err(format!("native registration changed in {}", path.display()).into());
	}

	let start = source.find(native).expect("checked registration");
	let end = source[start..]
		.find("\n\t});")
		.map(|offset| start + offset)
		.ok_or_else(|| format!("consumer closure changed in {}", path.display()))?;
	let request = if request {
		format!(
			"\n.add_systems(Startup, |mut localization: ResMut<Localization<texts::Translations>>| localization.load::<{scope}>())"
		)
	} else {
		String::new()
	};
	let adapted = format!(
		"{}.add_systems(Update, (move |{}\n\t}}).run_if(resource_exists::<{scope}>)){request};{}",
		&source[..start],
		&source[start + native.len()..end],
		&source[end + "\n\t});".len()..],
	);
	fs::write(path, adapted)?;
	Ok(())
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
