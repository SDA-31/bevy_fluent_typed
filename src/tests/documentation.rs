//! Public recipes must keep runtime and build dependency versions aligned.
#[test]
fn registry_setup_uses_matching_versions_without_source_overrides() {
	for (name, source) in [
		("README", include_str!("../../README.md")),
		("Rustdoc", include_str!("../../docs/quickstart.md")),
	] {
		let dependencies: Vec<_> = source
			.lines()
			.filter(|line| line.starts_with("bevy_fluent_typed ="))
			.collect();
		assert_eq!(dependencies.len(), 2, "{name}");

		for dependency in dependencies {
			assert!(
				dependency.contains(concat!("version = \"", env!("CARGO_PKG_VERSION"), "\"")),
				"{name}: {dependency}"
			);
			assert!(!dependency.contains("path ="), "{name}: {dependency}");
			assert!(!dependency.contains("git ="), "{name}: {dependency}");
		}

		assert!(!source.contains("[patch.crates-io]"), "{name}");
		assert!(source.contains("migration-0.2.md"), "{name}");
	}
}
