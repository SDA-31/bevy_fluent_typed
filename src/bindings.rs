//! Mode-independent bindings read the currently published scope resource.
use crate::bevy::prelude::*;
use crate::{FluentScope, LocalizedText};

pub(crate) fn refresh_ui<S: FluentScope>(
	catalog: Option<Res<S>>,
	mut texts: Query<(Ref<LocalizedText<S>>, &mut Text)>,
) {
	for (binding, mut text) in &mut texts {
		let value = match &catalog {
			Some(catalog) if catalog.is_changed() || binding.is_changed() => {
				binding.0.render(catalog)
			}
			Some(_) => continue,
			None => String::new(),
		};

		if text.0 != value {
			text.0 = value;
		}
	}
}

pub(crate) fn refresh_world<S: FluentScope>(
	catalog: Option<Res<S>>,
	mut texts: Query<(Ref<LocalizedText<S>>, &mut Text2d)>,
) {
	for (binding, mut text) in &mut texts {
		let value = match &catalog {
			Some(catalog) if catalog.is_changed() || binding.is_changed() => {
				binding.0.render(catalog)
			}
			Some(_) => continue,
			None => String::new(),
		};

		if text.0 != value {
			text.0 = value;
		}
	}
}
