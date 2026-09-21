//! Application-owned ICU services and locale-aware deferred text bindings.
use std::{collections::HashMap, sync::Arc};

use bevy_fluent_typed::bevy::{ecs as bevy_ecs, prelude::*};
use bevy_fluent_typed::{FluentCatalog, LocalizedText};
use icu_decimal::{DecimalFormatter, input::Decimal, options::GroupingStrategy};
use icu_experimental::dimension::percent::formatter::PercentFormatter;
use icu_locale_core::Locale as IcuLocale;
use writeable::Writeable;

use crate::texts::{Locale, Translations};

/// Application services, created once for each compiled catalog locale.
pub(crate) struct LocaleFormats {
	decimal: DecimalFormatter,
	percent: PercentFormatter<DecimalFormatter>,
}

/// Shared immutable services; cloning the Arc does not rebuild ICU formatters.
#[derive(Resource)]
pub(crate) struct NumberFormats(Arc<HashMap<Locale, LocaleFormats>>);

impl NumberFormats {
	/// Configure ICU in the application, without changing any localization-library API.
	pub(crate) fn try_new(grouping: GroupingStrategy) -> Result<Self, Box<dyn std::error::Error>> {
		let mut formats = HashMap::new();

		for &locale in Translations::locales() {
			// Numbering preferences are explicit; the message language remains `ar`.
			let language: IcuLocale = match locale {
				Locale::Ar => "ar-EG-u-nu-arab",
				_ => locale.as_ref(),
			}
			.parse()?;
			let decimal = DecimalFormatter::try_new((&language).into(), grouping.into())?;
			let percent = PercentFormatter::try_new_with_decimal_formatter(
				(&language).into(),
				decimal.clone(),
				Default::default(),
			)?;
			formats.insert(locale, LocaleFormats { decimal, percent });
		}

		Ok(Self(Arc::new(formats)))
	}
}

/// IDs let this headless example inspect the same entities across language changes.
#[derive(Resource)]
pub(crate) struct Labels {
	pub(crate) damage: Entity,
	pub(crate) chance: Entity,
}

/// Bevy injects the resource here. Closures capture shared services, not a borrowed Res.
pub(crate) fn spawn_labels(mut commands: Commands, formats: Res<NumberFormats>) {
	let damage = commands
		.spawn((
			Text::default(),
			damage_text("12345.60".parse().unwrap(), &formats),
		))
		.id();
	let chance = commands
		.spawn((
			Text2d::default(),
			chance_text("0.125".parse().unwrap(), &formats),
		))
		.id();
	commands.insert_resource(Labels { damage, chance });
}

/// Keep the original exact amount and select formatting from the current catalog.
pub(crate) fn damage_text(amount: Decimal, formats: &NumberFormats) -> LocalizedText<Translations> {
	let formats = Arc::clone(&formats.0);

	LocalizedText::new(move |catalog: &Translations| {
		let formatter = &formats[&catalog.locale()].decimal;
		let text = formatter.format_to_string(&amount);

		catalog.presentation().hud().msg_damage(text)
	})
}

/// The application stores a ratio; ICU4X 0.6's percent API takes percent units.
pub(crate) fn chance_text(ratio: Decimal, formats: &NumberFormats) -> LocalizedText<Translations> {
	let formats = Arc::clone(&formats.0);

	LocalizedText::new(move |catalog: &Translations| {
		let formatter = &formats[&catalog.locale()].percent;
		let mut percent = ratio.clone();
		percent.multiply_pow10(2); // Exact conversion: 0.125 of the whole is 12.5 percent.
		percent.trim_start(); // Scaling also moves visible leading padding; do not display 012.5%.
		let text = formatter.format(&percent).write_to_string().into_owned();

		catalog.presentation().hud().msg_chance(text)
	})
}
