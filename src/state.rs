//! Active language and immutable last-known-good catalogs of an arbitrary provider.
use crate::FluentCatalog;
use crate::bevy::{ecs as bevy_ecs, prelude::Resource};
use std::sync::Arc;

#[derive(Resource)]
/// Active language and independently replaceable last-known-good catalogs.
///
/// Initialized from embedded data. Mutating the resource triggers refresh of
/// localized text bindings; replacing an inactive locale does not select it.
pub struct Localization<C: FluentCatalog> {
	active: usize,
	catalogs: Vec<Arc<C>>,
	sources: Vec<Vec<(&'static str, String)>>,
}

impl<C: FluentCatalog> Default for Localization<C> {
	fn default() -> Self {
		Self::new(C::default_locale())
	}
}

impl<C: FluentCatalog> Localization<C> {
	/// Initialize all embedded catalogs and select a declared locale.
	///
	/// # Panics
	/// Panics if `locale` is absent from [`FluentCatalog::locales`].
	pub fn new(locale: C::Locale) -> Self {
		let active = index::<C>(locale);
		let catalogs = C::locales()
			.iter()
			.map(|&locale| Arc::new(C::embedded(locale)))
			.collect();

		let sources = C::locales()
			.iter()
			.map(|&locale| {
				C::modules(locale)
					.into_iter()
					.map(|module| (module.path, module.embedded.to_owned()))
					.collect()
			})
			.collect();

		Self {
			active,
			catalogs,
			sources,
		}
	}

	/// Return the currently selected language.
	pub fn locale(&self) -> C::Locale {
		C::locales()[self.active]
	}

	/// Select a language without discarding any valid loaded catalogs.
	///
	/// # Panics
	/// Panics if `locale` is absent from [`FluentCatalog::locales`].
	pub fn set_locale(&mut self, locale: C::Locale) {
		self.active = index::<C>(locale);
	}

	/// Borrow the active typed catalog for formatting at the presentation boundary.
	pub fn catalog(&self) -> &C {
		&self.catalogs[self.active]
	}

	/// Share the active immutable snapshot for identity-based resource publication.
	pub(crate) fn snapshot(&self) -> Arc<C> {
		Arc::clone(&self.catalogs[self.active])
	}

	/// Accept a fully validated locale without changing the active selection.
	/// Matching ordered sources preserve the previous Arc and resource identities.
	pub(crate) fn publish(
		&mut self,
		locale: C::Locale,
		catalog: Arc<C>,
		sources: &[(&'static str, String)],
	) {
		let index = index::<C>(locale);

		if self.sources[index] == sources {
			return;
		}

		self.sources[index] = sources.to_vec();
		self.catalogs[index] = catalog;
	}
}

fn index<C: FluentCatalog>(locale: C::Locale) -> usize {
	C::locales()
		.iter()
		.position(|candidate| *candidate == locale)
		.expect("catalog locale must belong to FluentCatalog::locales()")
}
