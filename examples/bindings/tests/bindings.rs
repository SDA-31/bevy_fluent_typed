use std::{
	process::{Command, Stdio},
	time::{Duration, Instant},
};

const PROCESS_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[test]
fn custom_component_and_field_bindings_switch_update_and_release_their_scope() {
	for arguments in [&[][..], &["--no-cache"][..]] {
		let stdout = run(
			env!("CARGO_BIN_EXE_localization-bindings-example"),
			arguments,
		);
		assert_eq!(
			stdout.lines().collect::<Vec<_>>(),
			[
				"en: Pilot Ada | Flight HUD | Pilot Ada",
				"es: Piloto Ada | Panel de vuelo | Piloto Ada",
				"ru: Пилот Ada | Панель полёта | Пилот Ada",
				"updated: Пилот Grace | Панель полёта | Пилот Grace",
				"remaining: Панель полёта",
				"unloaded",
			]
		);
	}
}

fn run(binary: &str, arguments: &[&str]) -> String {
	let mut child = Command::new(binary)
		.args(arguments)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.unwrap();
	let deadline = Instant::now() + PROCESS_TIMEOUT;

	while child.try_wait().unwrap().is_none() {
		if Instant::now() >= deadline {
			let _ = child.kill();
			let output = child.wait_with_output().unwrap();
			panic!("binding example did not finish: {output:?}");
		}

		std::thread::sleep(POLL_INTERVAL);
	}

	let output = child.wait_with_output().unwrap();
	assert!(output.status.success(), "{output:?}");
	String::from_utf8(output.stdout)
		.unwrap()
		.replace(['\u{2068}', '\u{2069}'], "")
}
