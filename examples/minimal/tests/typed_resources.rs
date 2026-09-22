use std::process::Command;

#[test]
fn typed_resources_quickstart_runs() {
	let output = Command::new(env!("CARGO_BIN_EXE_typed_resources"))
		.output()
		.unwrap();
	assert!(output.status.success(), "{:?}", output);
	let stdout = String::from_utf8(output.stdout)
		.unwrap()
		.replace(['\u{2068}', '\u{2069}'], "");
	assert_eq!(
		stdout.lines().collect::<Vec<_>>(),
		[
			"Flight HUD: Pilot Ada",
			"Flight HUD",
			"Panel de vuelo",
			"Piloto Ada"
		]
	);
}
