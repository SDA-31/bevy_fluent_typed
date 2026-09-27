//! Public mode selection and scheduling for independently loaded scopes.
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{
	FluentCatalog, Full, Lazy, LoadingMode, Localization, LocalizationManifest,
	assets::{ModuleAsset, ModuleLoader},
	compatibility,
	loading::{CatalogSource, reconcile, reload_catalogs, report_failures},
	resources,
	systems::CatalogReadiness,
};
use std::marker::PhantomData;

/// Ordering boundaries for localization consumers.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocalizationSystems {
	/// PreUpdate: handle requests/completions and publish ready resources.
	Publish,
	/// PostUpdate: refresh text after publication and before engine text layout.
	Refresh,
}

#[derive(Resource)]
struct Installed<C: FluentCatalog>(PhantomData<fn() -> C>);

/// Register loading and typed resources using an explicit immutable source contract.
///
/// Install after AssetPlugin. Full mode requests all modules of the selected locale;
/// Lazy mode waits for explicit requests. Creating this value performs no I/O.
/// Only one plugin/controller mode may own a given root provider in an App.
pub struct LocalizationPlugin<C: FluentCatalog, M: LoadingMode = Full> {
	manifest: LocalizationManifest,
	marker: PhantomData<fn() -> (C, M)>,
}

impl<C: FluentCatalog, M: LoadingMode> LocalizationPlugin<C, M> {
	/// Configure the selected loading mode from a ready source contract.
	/// File origins are Bevy asset addresses; named sources are preserved.
	pub fn new(manifest: LocalizationManifest) -> Self {
		Self {
			manifest,
			marker: PhantomData,
		}
	}
}

impl<C: FluentCatalog> LocalizationPlugin<C, Full> {
	/// Short form of `LocalizationPlugin::<C, Lazy>::new(manifest)`.
	pub fn new_lazy(manifest: LocalizationManifest) -> LocalizationPlugin<C, Lazy> {
		LocalizationPlugin::new(manifest)
	}
}

impl<C: FluentCatalog, M: LoadingMode> Plugin for LocalizationPlugin<C, M> {
	fn build(&self, app: &mut App) {
		assert!(
			!app.world().contains_resource::<Installed<C>>(),
			"one localization plugin per root provider is allowed"
		);
		let startup = C::locales()
			.iter()
			.copied()
			.find(|locale| locale.as_ref() == self.manifest.config().default_language);
		let error = if self.manifest.config().source_language != C::source_locale().as_ref() {
			Some("manifest source-language differs from the compiled schema".into())
		} else if startup.is_none() {
			Some("manifest default-language is not a compiled locale".into())
		} else {
			None
		};

		if !app.world().contains_resource::<Localization<C, M>>() {
			app.insert_resource(Localization::<C, M>::new(
				startup.unwrap_or_else(C::default_locale),
			));
		}

		app.insert_resource(Installed::<C>(PhantomData))
			.init_resource::<Localization<C, M>>()
			.insert_resource(CatalogSource::<C> {
				manifest: self.manifest.clone(),
				modules: C::modules(),
				error,
				marker: PhantomData,
			})
			.init_asset::<ModuleAsset<C>>()
			.register_asset_loader(ModuleLoader::<C>::new(&self.manifest));
		compatibility::register_notifications::<C>(app);

		for scope in C::scopes() {
			CatalogReadiness::register::<C, M>(app.world_mut(), &scope);
			(scope.bindings)(app);
		}

		app.add_systems(
			PreUpdate,
			(
				reload_catalogs::<C, M>,
				report_failures::<C, M>,
				reconcile::<C, M>,
				resources::synchronize::<C, M>,
			)
				.chain()
				.in_set(LocalizationSystems::Publish),
		)
		.add_systems(
			PostUpdate,
			(reconcile::<C, M>, resources::synchronize::<C, M>)
				.chain()
				.before(LocalizationSystems::Refresh),
		);
	}
}
