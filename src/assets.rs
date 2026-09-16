//! One aggregate asset watches all declared modules, publishing each language atomically.
use crate::{CatalogUpdate, FluentCatalog, Localization, ModuleSource, ReloadCatalogs, addresses};
use bevy::{
	asset::{AssetLoader, AssetPath, LoadContext, io::Reader},
	prelude::*,
};
use std::{any::type_name, io, marker::PhantomData, path::Path, sync::Arc};

#[derive(Asset)]
/// Aggregate load outcomes; individual rejected languages do not discard valid ones.
pub(crate) struct CatalogAsset<C: FluentCatalog> {
	candidates: Vec<Candidate<C>>,
}

struct Candidate<C: FluentCatalog> {
	locale: C::Locale,
	catalog: Result<Arc<C>, String>,
	sources: Vec<(&'static str, String)>,
}

impl<C: FluentCatalog> TypePath for CatalogAsset<C> {
	fn type_path() -> &'static str {
		type_name::<Self>()
	}

	fn short_type_path() -> &'static str {
		type_name::<Self>()
	}
}

pub(crate) struct CatalogLoader<C: FluentCatalog>(PhantomData<fn() -> C>);

impl<C: FluentCatalog> Default for CatalogLoader<C> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: FluentCatalog> TypePath for CatalogLoader<C> {
	fn type_path() -> &'static str {
		type_name::<Self>()
	}

	fn short_type_path() -> &'static str {
		type_name::<Self>()
	}
}

impl<C: FluentCatalog> AssetLoader for CatalogLoader<C> {
	type Asset = CatalogAsset<C>;
	type Settings = ();
	type Error = io::Error;

	/// Definition/address errors fail the aggregate; module read, UTF-8 and provider
	/// errors reject only the affected language. Continue reads after module failures
	/// so successfully read dependencies still participate in watching and recovery.
	async fn load(
		&self,
		reader: &mut dyn Reader,
		_: &(),
		context: &mut LoadContext<'_>,
	) -> Result<CatalogAsset<C>, io::Error> {
		let mut definition = Vec::new();
		reader.read_to_end(&mut definition).await?;
		let descriptor = C::descriptor(&definition).map_err(io::Error::other)?;
		addresses::validate_directory(&descriptor.modules_directory)?;

		let directory = context
			.path()
			.path()
			.parent()
			.unwrap_or(Path::new(""))
			.join(&descriptor.modules_directory);
		let mut candidates = Vec::with_capacity(C::locales().len());

		for &locale in C::locales() {
			addresses::validate_locale(locale.as_ref())?;
			let mut sources = Vec::new();
			let mut failures = Vec::new();

			for module in C::modules(locale) {
				addresses::validate_module(module.path)?;
				let path = directory.join(locale.as_ref()).join(module.path);
				let asset_path = AssetPath::from(path.clone())
					.with_source(context.path().source().clone_owned());
				// Successful byte reads register watcher dependencies, even if later
				// validation fails. Initially missing modules need an explicit reload.
				let bytes = match context.read_asset_bytes(asset_path).await {
					Ok(bytes) => bytes,
					Err(error) => {
						failures.push(format!("{}: {error}", path.display()));

						continue;
					}
				};
				let source = match String::from_utf8(bytes) {
					Ok(source) => source,
					Err(error) => {
						failures.push(format!("{}: {error}", path.display()));

						continue;
					}
				};

				sources.push((module.path, source));
			}

			let candidate = if failures.is_empty() {
				let borrowed: Vec<_> = sources
					.iter()
					.map(|(path, source)| ModuleSource { path, source })
					.collect();
				C::parse(locale, &borrowed).map(Arc::new)
			} else {
				Err(failures.join("\n"))
			};
			candidates.push(Candidate {
				locale,
				catalog: candidate,
				sources,
			});
		}

		Ok(CatalogAsset { candidates })
	}
}

#[derive(Resource)]
pub(crate) struct CatalogSource<C: FluentCatalog> {
	pub(crate) path: String,
	pub(crate) marker: PhantomData<fn() -> C>,
}

#[derive(Resource)]
pub(crate) struct CatalogHandle<C: FluentCatalog>(Handle<CatalogAsset<C>>);

pub(crate) fn load_catalogs<C: FluentCatalog>(
	mut commands: Commands,
	server: Res<AssetServer>,
	source: Res<CatalogSource<C>>,
) {
	commands.insert_resource(CatalogHandle::<C>(server.load(source.path.clone())));
}

pub(crate) fn reload_catalogs<C: FluentCatalog>(
	mut requests: MessageReader<ReloadCatalogs<C>>,
	server: Res<AssetServer>,
	source: Res<CatalogSource<C>>,
) {
	if requests.read().count() > 0 {
		server.reload(source.path.clone());
	}
}

/// Apply outcomes only for this provider's retained handle, preserving rejected locales.
pub(crate) fn publish_catalogs<C: FluentCatalog>(
	mut events: MessageReader<AssetEvent<CatalogAsset<C>>>,
	assets: Res<Assets<CatalogAsset<C>>>,
	handle: Option<Res<CatalogHandle<C>>>,
	source: Res<CatalogSource<C>>,
	mut localization: ResMut<Localization<C>>,
	mut updates: MessageWriter<CatalogUpdate<C>>,
) {
	let Some(handle) = handle else {
		return;
	};

	for event in events.read() {
		let (AssetEvent::Added { id } | AssetEvent::Modified { id }) = event else {
			continue;
		};

		if handle.0.id() != *id {
			continue;
		}

		let Some(asset) = assets.get(*id) else {
			continue;
		};

		for candidate in &asset.candidates {
			let locale = candidate.locale;

			match &candidate.catalog {
				Ok(catalog) => {
					localization.publish(locale, Arc::clone(catalog), &candidate.sources);
					updates.write(CatalogUpdate::Loaded { locale });
				}
				Err(error) => {
					updates.write(CatalogUpdate::Rejected {
						locale: Some(locale),
						path: source.path.clone(),
						error: error.clone(),
					});
				}
			}
		}
	}
}

/// Report aggregate failures for this provider's retained handle without replacing data.
pub(crate) fn report_failures<C: FluentCatalog>(
	mut events: MessageReader<bevy::asset::AssetLoadFailedEvent<CatalogAsset<C>>>,
	handle: Option<Res<CatalogHandle<C>>>,
	mut updates: MessageWriter<CatalogUpdate<C>>,
) {
	let Some(handle) = handle else {
		return;
	};

	for event in events.read() {
		if event.id == handle.0.id() {
			updates.write(CatalogUpdate::Rejected {
				locale: None,
				path: event.path.to_string(),
				error: event.error.to_string(),
			});
		}
	}
}
