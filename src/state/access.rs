//! Explicit locale selection that preserves existing module navigation and requests.
use crate::{
	FluentCatalog, FluentScope, Lazy, LoadingMode, Localization, Manual, ModuleLease, ModuleStatus,
	ModuleStore,
};

/// Borrowed access to one compiled language; inspection never starts loading.
pub struct LocaleView<'a, C: FluentCatalog> {
	store: &'a ModuleStore<C>,
}

impl<'a, C: FluentCatalog> LocaleView<'a, C> {
	/// Language selected by this view, independent of the game's active language.
	pub fn locale(&self) -> C::Locale {
		self.store.locale()
	}

	/// Reuse typed navigation and existing module-readiness errors.
	pub fn modules(&self) -> C::Modules<'a> {
		C::view(self.store)
	}

	/// Borrow a complete root when every module of this language is ready.
	pub fn catalog(&self) -> Option<&'a C> {
		self.store.get::<C>().ok()
	}

	/// Inspect the latest attempts without retaining the scope.
	pub fn status<S: FluentScope<Catalog = C>>(&self) -> ModuleStatus {
		self.store.status::<S>()
	}

	/// Inspect the selected language's per-scope counts without loading it.
	pub fn progress<S: FluentScope<Catalog = C>>(&self) -> crate::LoadingProgress<C::Locale> {
		self.store.progress::<S>()
	}
}

/// Borrowed fixed-language request interface; dropping the view does not remove pins.
pub struct LocaleViewMut<'a, C: FluentCatalog, M: LoadingMode = Lazy> {
	owner: &'a mut Localization<C, M>,
	index: usize,
}

impl<C: FluentCatalog, M: LoadingMode> Localization<C, M> {
	/// Inspect a compiled language without changing the active language or loading modules.
	///
	/// # Panics
	/// Panics if the locale is not declared by the provider.
	pub fn for_locale(&self, locale: C::Locale) -> LocaleView<'_, C> {
		assert!(
			C::locales().contains(&locale),
			"locale must belong to the provider"
		);
		LocaleView {
			store: self.locale_store(locale),
		}
	}

	/// Select a fixed language for explicit requests, without switching the game language.
	///
	/// # Panics
	/// Panics if the locale is not declared by the provider.
	pub fn for_locale_mut(&mut self, locale: C::Locale) -> LocaleViewMut<'_, C, M> {
		let index = self.locale_index(locale);
		LocaleViewMut { owner: self, index }
	}
}

macro_rules! explicit_locale_requests {
    ($($mode:ty),+) => { $(impl<C: FluentCatalog> LocaleViewMut<'_, C, $mode> {
        /// Retain this scope in the selected language until its fixed pin is removed.
        /// Repeated calls are idempotent and retry failed requests.
        pub fn load<S: FluentScope<Catalog=C>>(&mut self) { self.owner.fixed_load::<S>(self.index); }

        /// Remove only this scope's fixed-language manual pin.
        pub fn unload<S: FluentScope<Catalog=C>>(&mut self) { self.owner.fixed_unload::<S>(self.index); }

        /// Retain an independent fixed-language request until its token drops.
        pub fn hold<S: FluentScope<Catalog=C>>(&mut self) -> ModuleLease<S> { self.owner.fixed_hold::<S>(self.index) }
    })+ };
}

explicit_locale_requests!(Lazy, Manual);
