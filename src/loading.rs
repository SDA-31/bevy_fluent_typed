//! Reconcile logical requests with the selected language's concrete assets.
use crate::assets::ModuleAsset;
use crate::bevy::{
	asset::{AssetLoadFailedEvent, AssetPath},
	ecs as bevy_ecs,
	prelude::*,
};
use crate::compatibility::{MessageReader, MessageWriter};
use crate::state::RequestedModule;
use crate::{
	CatalogUpdate, FluentCatalog, LoadingMode, Localization, LocalizationManifest, Module,
	ModuleStatus, ReloadCatalogs, addresses,
};
use std::{marker::PhantomData, path::Path};

#[derive(Resource)]
pub(crate) struct CatalogSource<C: FluentCatalog> {
	pub(crate) manifest: LocalizationManifest,
	pub(crate) modules: Vec<Module<C>>,
	pub(crate) error: Option<String>,
	pub(crate) marker: PhantomData<fn() -> C>,
}

pub(crate) fn reload_catalogs<C: FluentCatalog, M: LoadingMode>(
	mut requests: MessageReader<ReloadCatalogs<C>>,
	mut localization: ResMut<Localization<C, M>>,
) {
	if requests.read().count() > 0 {
		let desired = localization.desired();
		localization.retry.extend(desired);
	}
}

pub(crate) fn reconcile<C: FluentCatalog, M: LoadingMode>(
	mut localization: ResMut<Localization<C, M>>,
	source: Res<CatalogSource<C>>,
	server: Res<AssetServer>,
	assets: Res<Assets<ModuleAsset<C>>>,
	mut updates: MessageWriter<CatalogUpdate<C>>,
) {
	let desired = localization.desired();
	let removed: Vec<_> = localization
		.entries
		.keys()
		.copied()
		.filter(|path| !desired.contains(path))
		.collect();

	for path in removed {
		localization.entries.remove(path);
		localization.retry.remove(path);
		localization.store.states.remove(path);
		if let Some(id) = localization.store.leaves.get(path).copied() {
			localization.store.values.remove(&id);
		}
	}

	let locale = localization.locale();

	for module in source
		.modules
		.iter()
		.filter(|module| desired.contains(module.path))
	{
		let path = module.path;
		let pending = localization
			.entries
			.get(path)
			.is_some_and(|entry| entry.pending);
		let retry = localization.retry.contains(path) && !pending;
		let initial = !localization.entries.contains_key(path);

		if retry {
			localization.retry.remove(path);
		}

		if initial || retry {
			localization
				.store
				.states
				.insert(path, ModuleStatus::Loading);

			if let Some(error) = &source.error {
				localization.entries.insert(
					path,
					RequestedModule {
						handle: None,
						accepted: None,
						pending: false,
					},
				);
				publish(&mut localization, path, Err(error.clone()), &mut updates);
				continue;
			}

			if source.manifest.embedded_modules().is_some() {
				localization.entries.insert(
					path,
					RequestedModule {
						handle: None,
						accepted: None,
						pending: false,
					},
				);
				let candidate = source
					.manifest
					.read(locale.as_ref(), path)
					.map_err(|error| error.to_string())
					.and_then(|bytes| (module.parse)(locale, &bytes));
				publish(&mut localization, path, candidate, &mut updates);
				continue;
			}

			match asset_address(&source.manifest, locale.as_ref(), path) {
				Ok(address) => {
					let previous = localization
						.entries
						.get(path)
						.and_then(|entry| entry.handle.clone());
					let handle = if let Some(handle) = previous {
						// load() would independently retry a Failed asset while reload()
						// is still queued on its detached task, starting two reads.
						server.reload(address);
						handle
					} else {
						server.load(address)
					};
					let accepted = localization
						.entries
						.get(path)
						.and_then(|entry| entry.accepted);
					localization.entries.insert(
						path,
						RequestedModule {
							handle: Some(handle),
							accepted,
							pending: true,
						},
					);
				}
				Err(error) => {
					localization.entries.insert(
						path,
						RequestedModule {
							handle: None,
							accepted: None,
							pending: false,
						},
					);
					publish(&mut localization, path, Err(error), &mut updates);
				}
			}
		}

		let Some(entry) = localization.entries.get(path) else {
			continue;
		};
		let Some(asset) = entry.handle.as_ref().and_then(|handle| assets.get(handle)) else {
			continue;
		};

		// Revisions order loader invocations, not completion. External overlapping
		// reads delayed before AssetLoader::load have no public Bevy request identity.
		if asset.locale != locale
			|| asset.path != path
			|| entry
				.accepted
				.is_some_and(|revision| asset.revision <= revision)
		{
			continue;
		}

		let entry = localization.entries.get_mut(path).unwrap();
		entry.accepted = Some(asset.revision);
		entry.pending = false;
		publish(
			&mut localization,
			path,
			asset.candidate.clone(),
			&mut updates,
		);
	}
}

fn publish<C: FluentCatalog, M: LoadingMode>(
	localization: &mut Localization<C, M>,
	path: &'static str,
	candidate: Result<crate::catalog::SharedScope, String>,
	updates: &mut MessageWriter<CatalogUpdate<C>>,
) {
	let locale = localization.locale();

	match candidate {
		Ok(value) => {
			localization.store.insert_leaf(path, value);
			updates.write(CatalogUpdate::Loaded {
				locale,
				path: path.into(),
			});
		}
		Err(error) => {
			localization
				.store
				.states
				.insert(path, ModuleStatus::Failed(error.clone()));
			updates.write(CatalogUpdate::Rejected {
				locale: Some(locale),
				path: path.into(),
				error,
			});
		}
	}
}

pub(crate) fn report_failures<C: FluentCatalog, M: LoadingMode>(
	mut events: MessageReader<AssetLoadFailedEvent<ModuleAsset<C>>>,
	mut localization: ResMut<Localization<C, M>>,
	mut updates: MessageWriter<CatalogUpdate<C>>,
) {
	let desired = localization.desired();

	for event in events.read() {
		let path = localization.entries.iter().find_map(|(path, entry)| {
			entry
				.handle
				.as_ref()
				.filter(|handle| desired.contains(path) && handle.id() == event.id)
				.map(|_| *path)
		});

		if let Some(path) = path {
			localization.entries.get_mut(path).unwrap().pending = false;
			publish(
				&mut localization,
				path,
				Err(event.error.to_string()),
				&mut updates,
			);
		}
	}
}

pub(crate) fn asset_address(
	manifest: &LocalizationManifest,
	locale: &str,
	module: &str,
) -> Result<AssetPath<'static>, String> {
	addresses::validate_locale(locale).map_err(|error| error.to_string())?;
	addresses::validate_module(module).map_err(|error| error.to_string())?;
	addresses::validate_directory(&manifest.config().languages_directory)
		.map_err(|error| error.to_string())?;
	let origin = manifest
		.file_path()
		.and_then(Path::to_str)
		.ok_or("file source requires a UTF-8 manifest origin")?;
	let origin = AssetPath::try_parse(origin).map_err(|error| error.to_string())?;

	if origin.label().is_some() {
		return Err("manifest origin cannot contain an asset label".into());
	}

	let path = origin
		.path()
		.parent()
		.unwrap_or_else(|| Path::new(""))
		.join(&manifest.config().languages_directory)
		.join(locale)
		.join(module);
	Ok(AssetPath::from(path).with_source(origin.source().clone_owned()))
}
