//! Compile the crate-page quick start using this consumer's renamed dependency.
extern crate localization_runtime as bevy_fluent_typed;

include!("../../../../docs/typed_resources.rs");

#[test]
fn typed_resources_quickstart_runs() {
	main();
}
