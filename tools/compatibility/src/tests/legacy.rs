use super::validate_dependencies;
use std::{fs, process::Command};

#[test]
fn tracks_legacy_facades_and_implementations_in_rustc_dependencies() {
	let fixture = tempfile::tempdir().unwrap();

	for (path, source) in [
		(
			"src/lib.rs",
			r#"#[cfg(any(feature = "bevy-0-16", feature = "bevy-0-17", feature = "bevy-0-18"))]
mod legacy_systems;
#[cfg(any(feature = "bevy-0-16", feature = "bevy-0-17", feature = "bevy-0-18"))]
mod compatibility;
"#,
		),
		("src/legacy_systems/mod.rs", "mod systems;\n"),
		("src/legacy_systems/systems.rs", ""),
		("src/compatibility/mod.rs", "mod readiness;\n"),
		("src/compatibility/readiness.rs", ""),
	] {
		let path = fixture.path().join(path);
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, source).unwrap();
	}

	for features in [
		"bevy-0-16",
		"bevy-0-17",
		"bevy-0-18",
		"bevy-0-19",
		"bevy-0-20",
		"build",
	] {
		let output = Command::new("rustc")
			.current_dir(fixture.path())
			.args([
				"--crate-name",
				"legacy_module_probe",
				"--crate-type",
				"lib",
				"--emit=dep-info=modules.d,metadata=runtime.rmeta",
				"--cfg",
			])
			.arg(format!("feature=\"{features}\""))
			.arg("src/lib.rs")
			.output()
			.unwrap();
		assert!(
			output.status.success(),
			"{}",
			String::from_utf8_lossy(&output.stderr)
		);

		let dependencies = fs::read_to_string(fixture.path().join("modules.d"))
			.unwrap()
			.replace('\\', "/");
		let expected = matches!(features, "bevy-0-16" | "bevy-0-17" | "bevy-0-18");
		validate_dependencies(&dependencies, features, expected).unwrap();
		assert!(validate_dependencies(&dependencies, features, !expected).is_err());
	}
}
