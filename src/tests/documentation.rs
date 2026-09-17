//! Public installation examples must track the package version, not local paths.

#[test]
fn registry_examples_use_the_manifest_version_in_both_dependency_sections() {
	let version = env!("CARGO_PKG_VERSION");
	let expected = format!("bevy_fluent_typed = {{ version = \"{version}\",");

	for (name, source) in [
		("README", include_str!("../../README.md")),
		(
			"Rustdoc quickstart",
			include_str!("../../docs/quickstart.md"),
		),
	] {
		let dependencies: Vec<_> = source
			.lines()
			.map(str::trim)
			.filter(|line| line.starts_with("bevy_fluent_typed ="))
			.collect();
		assert_eq!(dependencies.len(), 2, "{name}: runtime and build entries");

		for dependency in dependencies {
			assert!(dependency.starts_with(&expected), "{name}: {dependency}");
			assert!(!dependency.contains("path ="), "{name}: {dependency}");
		}
	}
}
