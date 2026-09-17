//! Checked parsing must be equally strict with or without the Bevy adapter.
use crate::{Locale, Translations};
use localization_runtime::FluentCatalog;

#[test]
fn plain_and_integrated_parsers_reject_changed_keys_variables_and_references() {
	let modules: Vec<_> = Translations::modules(Locale::En)
		.into_iter()
		.map(|module| (module.path, module.embedded))
		.collect();
	let index = modules
		.iter()
		.position(|(path, _)| *path == "presentation/hud.ftl")
		.unwrap();

	for original in super::newline_variants(modules[index].1) {
		assert_contract_changes(&modules, index, &original);
	}
}

fn assert_contract_changes(modules: &[(&str, &str)], index: usize, original: &str) {
	assert!(Translations::from_modules(Locale::En, modules).is_ok());
	let mut baseline = modules.to_vec();
	baseline[index].1 = original;
	assert!(Translations::from_modules(Locale::En, &baseline).is_ok());
	assert!(super::raw::Translations::from_modules(super::raw::Locale::En, &baseline).is_ok());

	for candidate in [
		original.replace("Pilot { $name }", "Pilot"),
		original.replace("$name", "$other"),
		original.replace("caption = { title }", "caption = Fixed caption"),
		original.replace("caption = { title }", "caption = { detail }"),
		original.replace("caption = { title }", "caption = { -absent }"),
		original.replace("Pilot { $name }", "Pilot { NUMBER($name) }"),
		original.replace("title = Flight HUD", ""),
		format!("{original}\nunexpected = New key\n"),
		format!("{original}\ntitle = Duplicate\n"),
		original.replace(
			".hint = Visible HUD",
			".hint = Visible HUD\n    .hint = Duplicate",
		),
		original.replace(
			"Press { $icon } to continue",
			"Press { $icon } { $icon } to continue",
		),
	] {
		assert_ne!(candidate, original, "fixture must change the contract");
		let mut sources = modules.to_vec();
		sources[index].1 = &candidate;
		assert!(
			Translations::from_modules(Locale::En, &sources).is_err(),
			"{candidate}"
		);
		assert!(
			super::raw::Translations::from_modules(super::raw::Locale::En, &sources).is_err(),
			"{candidate}"
		);
	}

	let candidate = original
		.replace("Flight HUD", "Changed HUD")
		.replace("Pilot", "Captain");
	let mut sources = modules.to_vec();
	sources[index].1 = &candidate;
	assert_eq!(
		Translations::from_modules(Locale::En, &sources)
			.unwrap()
			.presentation()
			.hud()
			.msg_title(),
		"Changed HUD"
	);
	assert_eq!(
		super::raw::Translations::from_modules(super::raw::Locale::En, &sources)
			.unwrap()
			.presentation()
			.hud()
			.msg_title(),
		"Changed HUD"
	);
}
