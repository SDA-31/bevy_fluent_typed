//! Generic Bevy scheduling; input, fonts, window titles and logs belong to the host.
use crate::bevy::{ecs as bevy_ecs, prelude::*, ui::UiSystems};
use crate::{
	CatalogUpdate, FluentCatalog, Localization, ReloadCatalogs,
	assets::{
		CatalogAsset, CatalogLoader, CatalogSource, load_catalogs, publish_catalogs,
		reload_catalogs, report_failures,
	},
	bindings::{refresh_ui, refresh_world},
	compatibility, resources,
};
use std::marker::PhantomData;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Scheduling hooks for application-specific reload reporting and text consumers.
pub enum LocalizationSystems {
	/// `PreUpdate`: process reloads, report outcomes and synchronize module resources.
	/// Order consumers after this set to observe its published resource snapshot.
	Publish,
	/// `PostUpdate`: synchronize resources after `Update` locale changes, then refresh
	/// bindings before Bevy's UI/world text layout.
	Refresh,
}

/// Register typed catalog loading and in-place text bindings for one provider.
///
/// Install after Bevy `AssetPlugin` (or `DefaultPlugins`). Insert a custom
/// [`Localization`] before adding this plugin to override its starting locale.
/// Bevy chooses the loader by catalog asset type; this does not register a
/// catch-all file extension or require a particular definition format.
/// Embedded resources are published immediately when the plugin is added;
/// asynchronous external loading starts during `Startup`.
///
/// Fonts, controls, window titles and display of [`CatalogUpdate`] errors belong
/// to the host. Enable feature `watch` and Bevy's watcher for automatic reloads.
pub struct LocalizationPlugin<C: FluentCatalog> {
	definition_path: String,
	marker: PhantomData<fn() -> C>,
}

impl<C: FluentCatalog> LocalizationPlugin<C> {
	/// Path to the provider's definition asset, relative to Bevy's asset root.
	///
	/// Named Bevy asset sources are preserved for loading dependent modules. The
	/// conventional filename is not hardcoded; the supplied path is authoritative.
	/// Its bytes are interpreted by [`FluentCatalog::descriptor`].
	/// This constructor only stores the path. Later load failures arrive through
	/// [`CatalogUpdate`], preserving embedded or last-known-good catalogs.
	pub fn new(definition_path: impl Into<String>) -> Self {
		Self {
			definition_path: definition_path.into(),
			marker: PhantomData,
		}
	}
}

impl<C: FluentCatalog> Plugin for LocalizationPlugin<C> {
	fn build(&self, app: &mut App) {
		app.init_resource::<Localization<C>>()
			.insert_resource(CatalogSource::<C> {
				path: self.definition_path.clone(),
				marker: PhantomData,
			})
			.init_asset::<CatalogAsset<C>>()
			.init_asset_loader::<CatalogLoader<C>>()
			.add_message::<CatalogUpdate<C>>()
			.add_message::<ReloadCatalogs<C>>()
			.add_systems(Startup, load_catalogs::<C>)
			.add_systems(
				PreUpdate,
				(
					reload_catalogs::<C>,
					publish_catalogs::<C>,
					report_failures::<C>,
					resources::synchronize::<C>,
				)
					.chain()
					.in_set(LocalizationSystems::Publish),
			)
			.add_systems(
				PostUpdate,
				(
					resources::synchronize::<C>,
					compatibility::before_text_detection((
						refresh_ui::<C>.before(UiSystems::Content),
						refresh_world::<C>.before(crate::bevy::sprite::update_text2d_layout),
					)),
				)
					.chain()
					.in_set(LocalizationSystems::Refresh),
			);

		resources::synchronize::<C>(app.world_mut());
	}
}
