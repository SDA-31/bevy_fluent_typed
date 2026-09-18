//! Decimal text and grammar follow the current generated catalog, not capture time.
use crate::{Locale, Translations};
use icu_decimal::{DecimalFormatter, input::Decimal};
use icu_locale_core::Locale as IcuLocale;
use icu_plurals::{PluralCategory, PluralRules};
use localization_runtime::bevy::prelude::*;
use localization_runtime::{
	FluentCatalog, Localization, LocalizationPlugin, LocalizedText, Message,
};

fn message(value: Decimal) -> Message<Translations> {
	let formatters: Vec<_> = Translations::locales()
		.iter()
		.map(|&locale| {
			let language: IcuLocale = locale.as_ref().parse().unwrap();
			let formatter =
				DecimalFormatter::try_new((&language).into(), Default::default()).unwrap();
			let rules = PluralRules::try_new_cardinal((&language).into()).unwrap();
			(locale, formatter, rules)
		})
		.collect();

	Message::new(move |catalog: &Translations| {
		let (_, formatter, rules) = formatters
			.iter()
			.find(|(locale, _, _)| *locale == catalog.locale())
			.expect("a formatter for every compiled language");
		let text = formatter.format_to_string(&value);
		let selector = match rules.category_for(&value) {
			PluralCategory::Zero => "zero",
			PluralCategory::One => "one",
			PluralCategory::Two => "two",
			PluralCategory::Few => "few",
			PluralCategory::Many => "many",
			PluralCategory::Other => "other",
		};

		catalog.numbers().msg_remaining(selector, text)
	})
}

#[test]
fn deferred_decimal_text_and_plural_category_follow_language_switches() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()))
		.add_plugins(LocalizationPlugin::<Translations>::new(
			"not-loaded-in-plural-test.toml",
		));
	let entity = app
		.world_mut()
		.spawn((
			Text::new(""),
			LocalizedText::from(message(Decimal::from(22))),
		))
		.id();
	app.finish();
	app.cleanup();

	for (locale, expected) in [
		(Locale::En, "22 items left"),
		(Locale::Es, "Quedan 22 elementos"),
		(Locale::Ru, "Осталось 22 предмета"),
		(Locale::En, "22 items left"),
	] {
		app.world_mut()
			.resource_mut::<Localization<Translations>>()
			.set_locale(locale);
		app.update();
		let actual = &app.world().get::<Text>(entity).unwrap().0;

		assert_eq!(actual.replace(['\u{2068}', '\u{2069}'], ""), expected);
	}
}

#[test]
fn decimal_visible_precision_and_native_numeric_selectors_remain_distinct() {
	let english = Locale::En.load();

	for (input, expected) in [("1", "1 item left"), ("1.0", "1.0 items left")] {
		let actual = message(input.parse().unwrap()).render(&english);
		assert_eq!(actual.replace(['\u{2068}', '\u{2069}'], ""), expected);
	}

	assert_eq!(english.numbers().msg_native(0), "Empty");
	assert_eq!(english.numbers().msg_native(1), "One item");
	assert_eq!(english.numbers().msg_native(2), "Several items");
}
