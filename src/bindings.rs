//! Refresh bound UI/world entities without respawning or touching editor state.
use crate::{FluentCatalog, Localization, LocalizedText};
use bevy::prelude::*;

pub(crate) fn refresh_ui<C: FluentCatalog>(
	localization: Res<Localization<C>>,
	mut texts: Query<(Ref<LocalizedText<C>>, &mut Text)>,
) {
	for (binding, mut text) in &mut texts {
		if !localization.is_changed() && !binding.is_changed() {
			continue;
		}

		let value = binding.0.render(localization.catalog());

		if text.0 != value {
			text.0 = value;
		}
	}
}

pub(crate) fn refresh_world<C: FluentCatalog>(
	localization: Res<Localization<C>>,
	mut texts: Query<(Ref<LocalizedText<C>>, &mut Text2d)>,
) {
	for (binding, mut text) in &mut texts {
		if !localization.is_changed() && !binding.is_changed() {
			continue;
		}

		let value = binding.0.render(localization.catalog());

		if text.0 != value {
			text.0 = value;
		}
	}
}
