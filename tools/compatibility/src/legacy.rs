//! Verify the legacy API and its implementation are restricted to older engines.
use crate::{Result, fixture::Fixture};
use serde_json::Value;
use std::{fs, path::Path};

pub(crate) fn check(fixture: &Fixture<'_>, backend: &str) -> Result<()> {
	let older = matches!(backend, "bevy-0-16" | "bevy-0-17" | "bevy-0-18");
	compiled_modules(fixture, backend, older)?;
	compiled_modules(fixture, "build", false)?;
	let source = r#"use localization_runtime::{IntoLocalizedSystems, LocalizationAppExt, localized};
use localization_runtime::bevy::prelude::*;
localization_runtime::translations!(mod texts);

fn hud(_: Res<texts::presentation::Hud>) {}

fn register<M>(app: &mut App, system: impl IntoLocalizedSystems<M>) {
    app.add_localized_systems(Update, system);
}

fn main() {
    let mut app = App::new();
    register(&mut app, hud);
    app.add_systems(Update, localized(hud).run_if(|| true));
    app.add_localized_startup_systems(hud);
}
"#;
	let probe = fixture
		.path
		.join("examples/minimal/src/bin/legacy_api_probe.rs");
	fs::write(fixture.path.join("legacy-api-probe.rs"), source)?;
	fs::write(&probe, source)?;
	let result = fixture.cargo(
		&[
			"check",
			"--locked",
			"-p",
			"localization-example",
			"--bin",
			"legacy_api_probe",
		],
		true,
	)?;
	fs::write(fixture.path.join("legacy-api-probe.log"), &result.stderr)?;
	let diagnostic = String::from_utf8_lossy(&result.stderr);
	let valid = if older {
		result.status.success()
	} else {
		!result.status.success()
			&& diagnostic.contains("error[E0432]")
			&& ["IntoLocalizedSystems", "LocalizationAppExt", "localized"]
				.iter()
				.all(|name| diagnostic.contains(&format!("no `{name}` in the root")))
	};

	if !valid {
		return Err(
			format!("unexpected legacy API availability on {backend}:\n{diagnostic}").into(),
		);
	}

	// Negative probes must not become targets of subsequent consumer checks.
	fs::remove_file(probe)?;
	Ok(())
}

fn compiled_modules(fixture: &Fixture<'_>, features: &str, expected: bool) -> Result<()> {
	let result = fixture.cargo(
		&[
			"check",
			"--locked",
			"-p",
			"bevy_fluent_typed",
			"--no-default-features",
			"--features",
			features,
			"--message-format=json",
		],
		true,
	)?;

	if !result.status.success() {
		return Err(String::from_utf8_lossy(&result.stderr).into_owned().into());
	}

	// Cargo reports canonical paths, while macOS temporary directories may use
	// a symlinked spelling. Resolve both before matching the compiler artifact.
	let manifest = fs::canonicalize(fixture.path.join("Cargo.toml"))?;
	let artifact = result
		.stdout
		.split(|byte| *byte == b'\n')
		.filter_map(|line| serde_json::from_slice::<Value>(line).ok())
		.find(|message| {
			message["reason"] == "compiler-artifact"
				&& message["target"]["name"] == "bevy_fluent_typed"
				&& message["manifest_path"]
					.as_str()
					.and_then(|path| fs::canonicalize(path).ok())
					.is_some_and(|path| path == manifest)
		})
		.ok_or("missing runtime compiler artifact for legacy module check")?;
	let output = artifact["filenames"]
		.as_array()
		.and_then(|files| {
			files
				.iter()
				.filter_map(Value::as_str)
				.find(|name| name.ends_with(".rmeta"))
		})
		.ok_or("missing runtime metadata artifact for legacy module check")?;
	let output = Path::new(output);
	let stem = output
		.file_stem()
		.and_then(|name| name.to_str())
		.and_then(|name| name.strip_prefix("lib"))
		.ok_or("unexpected runtime metadata filename")?;
	let dependencies = fs::read_to_string(output.with_file_name(format!("{stem}.d")))?;
	fs::write(
		fixture.path.join(format!("legacy-modules-{features}.log")),
		&dependencies,
	)?;
	let dependencies = dependencies.replace('\\', "/");
	validate_dependencies(&dependencies, features, expected)
}

fn validate_dependencies(dependencies: &str, features: &str, expected: bool) -> Result<()> {
	// These are rustc's actual source dependencies, not a scan of cfg annotations
	// or linked symbols that optimization could remove after compilation.
	for source in [
		"src/legacy_systems/mod.rs",
		"src/legacy_systems/systems.rs",
		"src/compatibility/readiness.rs",
	] {
		if dependencies.contains(source) != expected {
			return Err(format!("unexpected compilation of {source} with {features}").into());
		}
	}

	Ok(())
}

#[cfg(test)]
#[path = "tests/legacy.rs"]
mod tests;
