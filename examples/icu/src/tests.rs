use crate::{example_app, formatting, labels, texts};
use bevy_fluent_typed::Localization;
use bevy_fluent_typed::bevy::prelude::*;
use icu_decimal::{input::Decimal, options::GroupingStrategy};
use icu_experimental::dimension::percent::formatter::PercentFormatter;
use icu_locale_core::locale;
use writeable::Writeable;

fn switch(app: &mut App, locale: texts::Locale) {
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(locale);
	app.update();
}

fn plain(text: &str) -> String {
	// Compare content without removing the locale's significant separators/signs.
	text.replace(['\u{2068}', '\u{2069}'], "")
}

fn replace_damage(app: &mut App, value: &str) {
	let entity = app.world().resource::<formatting::Labels>().damage;
	let binding = formatting::damage_text(
		value.parse().unwrap(),
		app.world().resource::<formatting::NumberFormats>(),
	);
	app.world_mut().entity_mut(entity).insert(binding);
	app.update();
}

fn replace_chance(app: &mut App, value: &str) {
	let entity = app.world().resource::<formatting::Labels>().chance;
	let binding = formatting::chance_text(
		value.parse().unwrap(),
		app.world().resource::<formatting::NumberFormats>(),
	);
	app.world_mut().entity_mut(entity).insert(binding);
	app.update();
}

#[test]
fn existing_ui_and_world_entities_follow_the_catalog_locale() {
	let mut app = example_app().unwrap();
	let ids = app.world().resource::<formatting::Labels>();
	let original = (ids.damage, ids.chance);

	for (locale, damage, chance) in [
		(texts::Locale::En, "Damage: 12,345.60", "Chance: 12.5%"),
		(
			texts::Locale::Es,
			"Daño: 12.345,60",
			"Probabilidad: 12,5\u{a0}%",
		),
		(
			texts::Locale::Ru,
			"Урон: 12\u{a0}345,60",
			"Шанс: 12,5\u{a0}%",
		),
		(texts::Locale::En, "Damage: 12,345.60", "Chance: 12.5%"),
	] {
		switch(&mut app, locale);
		let actual = labels(&app);
		assert_eq!(plain(actual.0), damage);
		assert_eq!(plain(actual.1), chance);
		let ids = app.world().resource::<formatting::Labels>();
		assert_eq!((ids.damage, ids.chance), original);
	}
}

#[test]
fn arabic_uses_arabic_indic_digits_and_preserves_fluent_isolation() {
	let mut app = example_app().unwrap();
	switch(&mut app, texts::Locale::Ar);
	let (damage, chance) = labels(&app);
	assert_eq!(plain(damage), "الضرر: ١٢٬٣٤٥٫٦٠");
	// ICU4X 0.6's percent data uses ASCII % plus LRM for this locale, even with arab digits.
	assert_eq!(plain(chance), "الاحتمال: ١٢٫٥\u{200e}%\u{200e}");
	assert!(!chance.contains("0.125"));

	for text in [damage, chance] {
		assert!(text.contains('\u{2068}'));
		assert!(text.contains('\u{2069}'));
	}
}

#[test]
fn decimal_inputs_keep_large_integer_precision_visible_zeros_and_sign() {
	let mut app = example_app().unwrap();

	for (input, expected) in [
		("9007199254740993.01", "Damage: 9,007,199,254,740,993.01"),
		("-1234.50", "Damage: -1,234.50"),
		("1.0", "Damage: 1.0"),
		("0.00", "Damage: 0.00"),
	] {
		replace_damage(&mut app, input);
		assert_eq!(plain(labels(&app).0), expected);
	}

	switch(&mut app, texts::Locale::Ar);
	assert_eq!(plain(labels(&app).0), "الضرر: ٠٫٠٠");
}

#[test]
fn percentages_scale_ratios_exactly_once() {
	let mut app = example_app().unwrap();

	for (input, expected) in [
		("0", "Chance: 0%"),
		("0.01", "Chance: 1%"),
		("0.125", "Chance: 12.5%"),
		("1", "Chance: 100%"),
		("1.25", "Chance: 125%"),
		("-0.125", "Chance: -12.5%"),
	] {
		replace_chance(&mut app, input);
		assert_eq!(plain(labels(&app).1), expected);
	}
}

#[test]
fn percent_symbol_placement_is_owned_by_icu_not_a_rust_suffix() {
	let formatter = PercentFormatter::try_new(locale!("tr").into(), Default::default()).unwrap();
	let value: Decimal = "12.5".parse().unwrap();
	assert_eq!(formatter.format(&value).write_to_string(), "%12,5");
}

#[test]
fn changing_formatter_settings_requires_rebinding_captured_services() {
	let mut app = example_app().unwrap();
	app.insert_resource(formatting::NumberFormats::try_new(GroupingStrategy::Never).unwrap());
	app.update();
	assert_eq!(plain(labels(&app).0), "Damage: 12,345.60");

	// A locale update rerenders the old binding, but its captured Arc stays old.
	switch(&mut app, texts::Locale::Ru);
	assert_eq!(plain(labels(&app).0), "Урон: 12\u{a0}345,60");
	replace_damage(&mut app, "12345.60");
	assert_eq!(plain(labels(&app).0), "Урон: 12345,60");
	switch(&mut app, texts::Locale::En);
	assert_eq!(plain(labels(&app).0), "Damage: 12345.60");
}
