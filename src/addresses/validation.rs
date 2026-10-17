//! Asset-address safety for arbitrary providers; no filesystem discovery policy.
use std::{
	io,
	path::{Component, Path},
};

pub(crate) fn validate_directory(path: &Path) -> Result<(), io::Error> {
	let Some(text) = path.to_str() else {
		return Err(io::Error::other("catalog asset directory must be UTF-8"));
	};

	if text.is_empty()
		|| text.contains(['\\', '#', ':'])
		|| path
			.components()
			.any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
	{
		return Err(io::Error::other(format!(
			"invalid relative catalog asset address: {text:?}"
		)));
	}

	Ok(())
}

pub(crate) fn validate_locale(locale: &str) -> Result<(), io::Error> {
	validate_directory(Path::new(locale))?;

	if locale.contains('/') || locale == "." {
		return Err(io::Error::other(
			"catalog locale must be one directory name",
		));
	}

	Ok(())
}

pub(crate) fn validate_module(module: &str) -> Result<(), io::Error> {
	validate_directory(Path::new(module))?;

	if module.ends_with('/') || Path::new(module).file_name().is_none() {
		return Err(io::Error::other("catalog module must name a file"));
	}

	Ok(())
}

#[cfg(test)]
#[path = "tests/validation.rs"]
mod tests;
