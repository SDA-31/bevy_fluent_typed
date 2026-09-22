use std::process::Command;

#[test]
fn generated_resources_switch_languages() {
	let output = Command::new(env!("CARGO_BIN_EXE_localization-codegen-example"))
		.output()
		.unwrap();
	assert!(output.status.success(), "{:?}", output);
	let stdout = String::from_utf8(output.stdout).unwrap();
	assert_eq!(
		stdout.lines().collect::<Vec<_>>(),
		["Hello!", "¡Hola!", "Привет!"]
	);
}
