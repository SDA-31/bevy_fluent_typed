use std::process::Command;

#[test]
fn handwritten_provider_updates_text_bindings() {
	let output = Command::new(env!("CARGO_BIN_EXE_localization-no-codegen-example"))
		.output()
		.unwrap();
	assert!(output.status.success(), "{:?}", output);
	let stdout = String::from_utf8(output.stdout).unwrap();
	assert_eq!(
		stdout.lines().collect::<Vec<_>>(),
		["Hello!", "¡Hola!", "Привет!"]
	);
}
