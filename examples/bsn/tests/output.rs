use std::{
	process::{Command, Stdio},
	time::{Duration, Instant},
};

// Bound a broken headless runner without adding test deadlines to the example.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[test]
fn bsn_example_waits_for_localized_text_and_exits_successfully() {
	let mut child = Command::new(env!("CARGO_BIN_EXE_localization-bsn-example"))
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.unwrap();
	let deadline = Instant::now() + PROCESS_TIMEOUT;

	while child.try_wait().unwrap().is_none() {
		if Instant::now() >= deadline {
			let _ = child.kill();
			let output = child.wait_with_output().unwrap();
			panic!("example did not finish before timeout: {output:?}");
		}

		std::thread::sleep(POLL_INTERVAL);
	}

	let output = child.wait_with_output().unwrap();
	assert!(output.status.success(), "{:?}", output);
	let text = String::from_utf8(output.stdout)
		.unwrap()
		.replace(['\u{2068}', '\u{2069}'], "");
	assert_eq!(text.trim(), "Hello, Ada!");
}
