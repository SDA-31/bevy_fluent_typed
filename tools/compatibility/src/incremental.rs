//! Exercise real Cargo freshness in the disposable explicit-build consumer.
use crate::{Result, fixture::Fixture};
use serde_json::Value;
use std::fs;

pub(crate) fn check(fixture: &Fixture<'_>) -> Result<()> {
	let consumer = fixture.path.join("examples/codegen");
	let translations = consumer.join("assets/localizations/translations");
	let arguments = [
		"check",
		"--locked",
		"-p",
		"localization-codegen-example",
		"--message-format=json",
	];
	check_result(fixture, &arguments, None)?;
	check_result(fixture, &arguments, Some(true))?;
	check_result(fixture, &arguments, Some(true))?;

	for locale in ["en", "es", "ru"] {
		fs::write(
			translations.join(locale).join("extra.ftl"),
			"added = New module\n",
		)?;
	}

	check_result(fixture, &arguments, Some(false))?;
	fs::remove_file(translations.join("es/extra.ftl"))?;
	expect_failure(fixture, &arguments, "extra.ftl")?;
	fs::write(translations.join("es/extra.ftl"), "added = New module\n")?;
	check_result(fixture, &arguments, Some(false))?;

	for locale in ["en", "es", "ru"] {
		fs::remove_file(translations.join(locale).join("extra.ftl"))?;
	}

	check_result(fixture, &arguments, Some(false))?;
	fs::create_dir_all(translations.join("fr/ui"))?;
	fs::write(
		translations.join("fr/ui/greeting.ftl"),
		"hello = Bonjour !\n",
	)?;
	check_result(fixture, &arguments, Some(false))?;
	fs::remove_file(translations.join("fr/ui/greeting.ftl"))?;
	fs::remove_dir(translations.join("fr/ui"))?;
	fs::remove_dir(translations.join("fr"))?;
	check_result(fixture, &arguments, Some(false))?;
	fs::write(
		translations.join("en/ui/greeting.ftl"),
		"hello = Updated greeting\n",
	)?;
	check_result(fixture, &arguments, Some(false))?;
	check_unchanged(fixture, &arguments)?;

	let manifest_path = consumer.join("Cargo.toml");
	let manifest = fs::read_to_string(&manifest_path)?;
	fs::write(
		&manifest_path,
		manifest.replacen("[package]", "[package]\nbuild = false", 1),
	)?;
	expect_failure(fixture, &arguments, "OUT_DIR")?;
	fs::write(&manifest_path, &manifest)?;
	check_result(fixture, &arguments, Some(false))?;

	// The bridge explicitly tracks source directories even if Cargo excludes assets.
	fs::write(
		&manifest_path,
		manifest.replacen("[package]", "[package]\nexclude = [\"assets/**\"]", 1),
	)?;
	check_result(fixture, &arguments, Some(false))?;

	for locale in ["en", "es", "ru"] {
		fs::write(
			translations.join(locale).join("tracked.ftl"),
			"title = Tracked module\n",
		)?;
	}

	check_result(fixture, &arguments, Some(false))?;
	check_unchanged(fixture, &arguments)?;
	println!(
		"PASS explicit build generation: edits, module/locale discovery, failures, recovery, excluded assets and unchanged rebuilds"
	);
	Ok(())
}

fn check_unchanged(fixture: &Fixture<'_>, arguments: &[&str]) -> Result<()> {
	// Upstream watches staged inputs written during build.rs. Their timestamp can
	// exceed Cargo's build-start timestamp, causing one extra build after an edit.
	// Accept that documented limitation, but never allow a perpetual rebuild loop.
	if !check_result(fixture, arguments, None)? {
		println!("NOTE: one settling rebuild for upstream staging timestamps");
	}

	check_result(fixture, arguments, Some(true))?;
	check_result(fixture, arguments, Some(true))?;
	Ok(())
}

fn check_result(fixture: &Fixture<'_>, arguments: &[&str], fresh: Option<bool>) -> Result<bool> {
	let output = fixture.cargo(arguments, true)?;

	if !output.status.success() {
		return Err(format!(
			"incremental check failed:\n{}\n{}",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		)
		.into());
	}

	let mut artifact = None;
	let mut build_facade = false;
	let mut runtime_facade = false;

	for line in String::from_utf8(output.stdout)?.lines() {
		let message: Value = serde_json::from_str(line)?;

		if message["reason"] == "compiler-artifact"
			&& message["target"]["name"] == "bevy_fluent_typed"
		{
			let features = message["features"]
				.as_array()
				.ok_or("facade features missing")?;

			if features.iter().any(|feature| feature == "build") {
				if features.len() != 1 {
					return Err(format!(
						"runtime features leaked into the build facade: {features:?}"
					)
					.into());
				}

				build_facade = true;
			} else if features.iter().any(|feature| feature == "runtime") {
				runtime_facade = true;
			}
		}

		if message["reason"] == "compiler-artifact"
			&& message["target"]["name"] == "localization-codegen-example"
		{
			artifact = message["fresh"].as_bool();
		}
	}

	if !build_facade || !runtime_facade {
		return Err("separate build-only and runtime facade artifacts were not emitted".into());
	}

	let actual = artifact.ok_or("consumer compiler-artifact missing from Cargo JSON output")?;

	if let Some(expected) = fresh
		&& actual != expected
	{
		return Err(format!("consumer freshness: expected {expected}, got {actual}").into());
	}

	Ok(actual)
}

fn expect_failure(fixture: &Fixture<'_>, arguments: &[&str], diagnostic: &str) -> Result<()> {
	let output = fixture.cargo(arguments, true)?;
	let stdout = String::from_utf8_lossy(&output.stdout);
	let stderr = String::from_utf8_lossy(&output.stderr);

	if output.status.success() || !(stdout.contains(diagnostic) || stderr.contains(diagnostic)) {
		return Err(
			format!("expected failure containing {diagnostic}:\n{stdout}\n{stderr}").into(),
		);
	}

	Ok(())
}
