//! Unreleased examples must resolve matching source revisions without sibling paths.
#[test]
fn development_setup_pins_matching_generator_in_both_dependency_graphs() {
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
				dependency.contains("feat/runtime-module-loading"),
				"{name}: {dependency}"
			);
			assert!(!dependency.contains("path ="));
		}
		assert!(source.contains("0e5ed447cc08553818a240fc6cfe736aa67fb4b3"));
		assert!(source.contains("Unreleased"));
	}
}
