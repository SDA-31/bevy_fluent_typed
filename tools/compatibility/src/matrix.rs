//! Resolve exact release families and test consumers in separate Cargo graphs.
use crate::{Options, Result, fixture::Fixture};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

pub(crate) fn backend(version: &str) -> Result<String> {
	let parts: Vec<_> = version.split('.').collect();

	if parts.len() != 3
		|| parts[0] != "0"
		|| !matches!(parts[1], "16" | "17" | "18" | "19")
		|| parts[2].is_empty()
		|| !parts[2].bytes().all(|byte| byte.is_ascii_digit())
		|| parts[2].parse::<u32>().is_err()
		|| (parts[1] == "16" && parts[2].parse::<u32>() == Ok(0))
	{
		return Err(format!("unsupported exact stable release: {version}").into());
	}

	Ok(format!("bevy-0-{}", parts[1]))
}

pub(crate) fn check(source: &Path, options: &Options, version: &str, host: &str) -> Result<()> {
	let backend = backend(version)?;
	let fixture = Fixture::new(source, options, version)?;
	fixture.pin(&["bevy".into()], version)?;
	let features = format!("{backend},codegen,watch");
	let args = [
		"metadata",
		"--format-version=1",
		"--filter-platform",
		host,
		"--no-default-features",
		"--features",
		&features,
	];
	let graph = metadata(&fixture, &args)?;
	let active = dependency_tree(&fixture, "localization-example", "")?;
	let family = official_packages(&graph)
		.filter(|package| {
			active.contains(&(
				package["name"].as_str().unwrap().to_owned(),
				package["version"].as_str().unwrap().to_owned(),
			))
		})
		.map(|package| package["name"].as_str().unwrap().to_owned())
		.collect::<BTreeSet<_>>();
	fixture.pin(&family.into_iter().collect::<Vec<_>>(), version)?;
	let graph = metadata(&fixture, &args)?;
	let active = dependency_tree(&fixture, "localization-example", "")?;
	let official: BTreeSet<_> = official_packages(&graph)
		.map(|package| package["name"].as_str().unwrap())
		.collect();

	for (name, resolved) in &active {
		let expected = package_version(name, version);

		if official.contains(name.as_str()) && resolved != expected {
			return Err(
				format!("wrong engine package: {name} {resolved} (expected {expected})").into(),
			);
		}
	}

	fs::write(
		fixture.path.join("verified-metadata.json"),
		serde_json::to_vec_pretty(&graph)?,
	)?;
	let runtime = dependency_tree(&fixture, "bevy_fluent_typed", &format!("{backend},watch"))?;
	let host = dependency_tree(&fixture, "bevy_fluent_typed", "build")?;

	if host
		.iter()
		.any(|(name, _)| name == "bevy" || name.starts_with("bevy_0_") || name == "bevy_internal")
	{
		return Err("build-only facade unexpectedly depends on Bevy".into());
	}

	if !host.iter().any(|(name, _)| name == "fluent_typed_codegen") {
		return Err("build facade is missing its generator".into());
	}

	fixture.success(&[
		"check",
		"--locked",
		"-p",
		"bevy_fluent_typed",
		"--no-default-features",
		"--features",
		"build",
	])?;
	let target_graph = fixture.cargo(
		&[
			"tree",
			"--locked",
			"-p",
			"localization-codegen-example",
			"--edges",
			"normal,no-proc-macro",
			"--prefix",
			"none",
			"--format",
			"{p}",
		],
		true,
	)?;

	if !target_graph.status.success() {
		return Err(String::from_utf8_lossy(&target_graph.stderr)
			.into_owned()
			.into());
	}

	let target_graph = String::from_utf8(target_graph.stdout)?;

	for line in target_graph.lines() {
		if matches!(
			line.split_whitespace().next(),
			Some("fluent_typed_codegen" | "prettyplease" | "tempfile")
		) {
			return Err(
				format!("host-only generator dependency entered target graph: {line}").into(),
			);
		}
	}

	if runtime.iter().any(|(name, _)| {
		matches!(
			name.as_str(),
			"bevy_fluent_codegen_bridge" | "fluent_typed_codegen"
		)
	}) {
		return Err("runtime without codegen unexpectedly depends on the bridge/generator".into());
	}

	fixture.success(&[
		"test",
		"--locked",
		"-p",
		"bevy_fluent_typed",
		"--no-default-features",
		"--features",
		&format!("{backend},watch"),
	])?;
	let example = ["--locked", "-p", "localization-example"];
	fixture.success(&[&["test"][..], &example].concat())?;

	for binary in ["localization-example", "typed_resources"] {
		fixture.success(&[&["run"][..], &example, &["--bin", binary]].concat())?;
	}

	fixture.success(&["test", "--locked", "-p", "localization-codegen-example"])?;
	fixture.success(&[
		"test",
		"--locked",
		"-p",
		"localization-no-codegen-example",
		"--no-default-features",
		"--features",
		&backend,
	])?;

	crate::incremental::check(&fixture)?;

	if backend == "bevy-0-19" {
		// Verify root, group and leaf without making the consumer forward backend
		// features to its build dependency merely to cfg-gate a test.
		fs::write(
			fixture
				.path
				.join("examples/minimal/src/bin/immutable_types_probe.rs"),
			r#"use localization_runtime::bevy::{ecs::component::Immutable, prelude::*};
localization_runtime::translations!(mod texts);

fn immutable<T: Resource + Component<Mutability = Immutable>>() {}

fn main() {
    immutable::<texts::Translations>();
    immutable::<texts::Presentation>();
    immutable::<texts::presentation::Hud>();
}
"#,
		)?;
		fixture.success(
			&[
				&["check"][..],
				&example,
				&["--bin", "immutable_types_probe"],
			]
			.concat(),
		)?;
	}

	fs::write(
		fixture
			.path
			.join("examples/minimal/src/bin/immutable_probe.rs"),
		"localization_runtime::translations!(mod texts);\nfn mutable(_: localization_runtime::bevy::prelude::ResMut<texts::Translations>) {}\nfn main() {}\n",
	)?;
	let probe = fixture.cargo(
		&[&["check"][..], &example, &["--bin", "immutable_probe"]].concat(),
		true,
	)?;
	fs::write(fixture.path.join("immutability-probe.log"), &probe.stderr)?;
	let diagnostic = String::from_utf8_lossy(&probe.stderr);
	let valid = if backend == "bevy-0-19" {
		!probe.status.success() && immutable_resource_error(&diagnostic)
	} else {
		probe.status.success()
	};

	if !valid {
		return Err(format!("unexpected resource mutability result:\n{diagnostic}").into());
	}

	if backend == "bevy-0-19" {
		for (features, expected, log) in [
			("", "select one Bevy backend", "missing-backend.log"),
			(
				"bevy-0-17,bevy-0-19",
				"Bevy backends are mutually exclusive",
				"conflicting-backends.log",
			),
		] {
			let result = fixture.cargo(
				&[
					"check",
					"--locked",
					"-p",
					"bevy_fluent_typed",
					"--no-default-features",
					"--features",
					features,
				],
				true,
			)?;
			fs::write(fixture.path.join(log), &result.stderr)?;

			if result.status.success()
				|| !String::from_utf8_lossy(&result.stderr).contains(expected)
			{
				return Err(format!(
					"backend selection probe did not produce {expected}: {}",
					String::from_utf8_lossy(&result.stderr)
				)
				.into());
			}
		}
	}

	println!("PASS {version}: runtime, generated resources, examples and mutability contract");
	Ok(())
}

/// Bevy 0.16.1's published renderer requires the independently patched color crate.
pub(crate) fn package_version<'a>(name: &str, version: &'a str) -> &'a str {
	if name == "bevy_color" && version == "0.16.1" {
		"0.16.2"
	} else {
		version
	}
}

fn immutable_resource_error(diagnostic: &str) -> bool {
	// Rust may describe only the unsatisfied bound, without spelling Immutable.
	diagnostic.contains("error[E0271]")
		&& diagnostic.contains("<Translations as Component>::Mutability == Mutable")
		&& diagnostic.contains("required by a bound in `ResMut`")
}

fn metadata(fixture: &Fixture<'_>, arguments: &[&str]) -> Result<Value> {
	let result = fixture.cargo(arguments, true)?;

	if !result.status.success() {
		return Err(String::from_utf8_lossy(&result.stderr).into_owned().into());
	}

	Ok(serde_json::from_slice(&result.stdout)?)
}

fn official_packages(graph: &Value) -> impl Iterator<Item = &Value> {
	graph["packages"]
		.as_array()
		.unwrap()
		.iter()
		.filter(|package| {
			// This official proc-macro package omits repository metadata in some
			// engine releases; it must still participate in exact release pinning.
			if package["name"] == "bevy_ecs_macros" && package["source"].is_string() {
				return true;
			}

			package["repository"].as_str().is_some_and(|repository| {
				repository.trim_end_matches('/').trim_end_matches(".git")
					== "https://github.com/bevyengine/bevy"
			})
		})
}

fn dependency_tree(
	fixture: &Fixture<'_>,
	package: &str,
	features: &str,
) -> Result<BTreeSet<(String, String)>> {
	// Metadata resolves every workspace member; tree's package selection describes
	// the consumer actually being compiled, without another member's defaults.
	let result = fixture.cargo(
		&[
			"tree",
			"-p",
			package,
			"--no-default-features",
			"--features",
			features,
			"--prefix",
			"none",
			"--edges",
			"normal,build",
			"--format",
			"{p}",
		],
		true,
	)?;

	if !result.status.success() {
		return Err(String::from_utf8_lossy(&result.stderr).into_owned().into());
	}

	fs::write(
		fixture.path.join(format!("{package}-dependencies.txt")),
		&result.stdout,
	)?;
	let text = String::from_utf8(result.stdout)?;
	let mut packages = BTreeSet::new();

	for line in text.lines() {
		let mut fields = line.split_whitespace();
		let name = fields.next().ok_or("missing package name in cargo tree")?;
		let version = fields
			.next()
			.and_then(|value| value.strip_prefix('v'))
			.ok_or("missing package version in cargo tree")?;
		packages.insert((name.into(), version.into()));
	}

	Ok(packages)
}

#[cfg(test)]
mod tests {
	#[test]
	fn color_patch_is_specific_to_the_published_0161_release_family() {
		assert_eq!(super::package_version("bevy_color", "0.16.1"), "0.16.2");
		assert_eq!(super::package_version("bevy_ecs", "0.16.1"), "0.16.1");
		assert_eq!(super::package_version("bevy_color", "0.17.0"), "0.17.0");
	}

	#[test]
	fn mutability_probe_requires_the_actual_resource_bound_failure() {
		assert!(super::immutable_resource_error(
			"error[E0271]: type mismatch resolving `<Translations as Component>::Mutability == Mutable`\nnote: required by a bound in `ResMut`"
		));
		assert!(!super::immutable_resource_error(
			"error[E0433]: could not find Immutable or Mutable"
		));
	}

	#[test]
	fn official_family_includes_unlabelled_ecs_macros_but_not_separate_projects() {
		let graph = serde_json::json!({ "packages": [
			{"name": "bevy", "repository": "https://github.com/bevyengine/bevy"},
			{"name": "bevy_ecs_macros", "source": "registry+https://github.com/rust-lang/crates.io-index"},
			{"name": "bevy_mikktspace", "repository": "https://github.com/bevyengine/bevy_mikktspace"},
			{"name": "bevy_fluent_typed", "repository": "https://github.com/SDA-31/bevy_fluent_typed"}
		] });
		let names: Vec<_> = super::official_packages(&graph)
			.map(|package| package["name"].as_str().unwrap())
			.collect();
		assert_eq!(names, ["bevy", "bevy_ecs_macros"]);
	}

	#[test]
	fn runner_accepts_concrete_releases_within_supported_backend_ranges() {
		// The runner pins one reproducible release; library requirements admit patches.
		for version in ["0.16.1", "0.17.0", "0.17.3", "0.18.1", "0.19.1"] {
			assert!(super::backend(version).is_ok());
		}

		for version in [
			"0.15.0",
			"0.16.0",
			"0.20.0",
			"0.19",
			"0.19.*",
			"0.19.+1",
			"0.19.0-rc.1",
			"0.19.0\n",
		] {
			assert!(super::backend(version).is_err());
		}
	}
}
