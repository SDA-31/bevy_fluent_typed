//! Maintainer command: isolated exact-version tests, not a library dependency.
mod fixture;
mod incremental;
mod matrix;

use std::{env, error::Error, path::PathBuf, process::Command};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Options {
	generator: PathBuf,
	target: Option<PathBuf>,
	offline: bool,
	versions: Vec<String>,
}

impl Options {
	fn parse() -> Result<Self> {
		let mut arguments = env::args().skip(1);
		let mut generator = None;
		let mut target = None;
		let mut offline = false;
		let mut versions = Vec::new();

		while let Some(argument) = arguments.next() {
			match argument.as_str() {
				"--generator" => {
					generator = Some(
						PathBuf::from(arguments.next().ok_or("missing generator path")?)
							.canonicalize()?,
					)
				}
				"--target-dir" => {
					let path = PathBuf::from(arguments.next().ok_or("missing target directory")?);
					target = Some(env::current_dir()?.join(path));
				}
				"--offline" => offline = true,
				"--help" | "-h" => {
					println!(
						"Usage: cargo run -p bevy-fluent-compatibility -- --generator PATH [--offline] [--target-dir PATH] 0.16.1 0.17.0 0.18.0 0.19.0 0.20.0-rc.1"
					);
					std::process::exit(0);
				}
				_ if argument.starts_with('-') => {
					return Err(format!("unknown option: {argument}").into());
				}
				_ => versions.push(argument),
			}
		}

		if versions.is_empty() {
			return Err("provide at least one supported exact Bevy version".into());
		}

		for version in &versions {
			matrix::backend(version)?;
		}

		Ok(Self {
			generator: generator
				.ok_or("--generator PATH is required to verify an explicit generator checkout")?,
			target,
			offline,
			versions,
		})
	}
}

fn main() -> Result<()> {
	let options = Options::parse()?;
	let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../..")
		.canonicalize()?;
	let compiler = Command::new("rustc").arg("-vV").output()?;
	let compiler = String::from_utf8(compiler.stdout)?;
	let host = compiler
		.lines()
		.find_map(|line| line.strip_prefix("host: "))
		.ok_or("rustc did not report its host")?;

	for version in &options.versions {
		matrix::check(&source, &options, version, host)?;
	}

	Ok(())
}
