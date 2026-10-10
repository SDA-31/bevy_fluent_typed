//! Reconcile logical requests with the selected language's concrete assets.
#[cfg(feature = "manifest")]
use crate::assets::{ModuleAsset, PreparedModuleAsset};
use crate::bevy::prelude::*;
#[cfg(feature = "manifest")]
use crate::bevy::{
	asset::{AssetEvent, AssetLoadFailedEvent, AssetPath},
	ecs as bevy_ecs,
};
use crate::compatibility::{MessageReader, MessageWriter};
#[cfg(feature = "manifest")]
use crate::state::RequestedModule;
use crate::{
	CatalogUpdate, FluentCatalog, LoadingMode, Localization, ModuleStatus, ReloadCatalogs,
};
#[cfg(feature = "manifest")]
use crate::{LocalizationManifest, Module, addresses};
#[cfg(feature = "manifest")]
use std::{collections::HashMap, marker::PhantomData, path::Path};

#[cfg(feature = "manifest")]
#[derive(Resource)]
pub(crate) struct CatalogSource<C: FluentCatalog> {
	pub(crate) manifest: LocalizationManifest,
	pub(crate) attempts: crate::assets::PreparationAttempts<C>,
	pub(crate) modules: HashMap<&'static str, Module<C>>,
	pub(crate) error: Option<String>,
	pub(crate) marker: PhantomData<fn() -> C>,
}

pub(crate) fn reload_catalogs<C: FluentCatalog, M: LoadingMode>(
	mut requests: MessageReader<ReloadCatalogs<C>>,
	mut localization: ResMut<Localization<C, M>>,
) {
	if requests.read().count() > 0 {
		let desired = localization.desired();
		localization.retry.extend(desired.iter().copied());
		localization.commit_requested = false;

		if let Some(preparation) = localization.preparation.as_mut() {
			preparation.retry.extend(desired.iter().copied());
		}
	}
}

#[cfg(feature = "manifest")]
#[derive(crate::bevy::ecs::system::SystemParam)]
pub(crate) struct CatalogAssets<'w, 's, C: FluentCatalog> {
	server: Res<'w, AssetServer>,
	loaded: Res<'w, Assets<ModuleAsset<C>>>,
	prepared: Res<'w, Assets<PreparedModuleAsset<C>>>,
	changes: MessageReader<'w, 's, AssetEvent<ModuleAsset<C>>>,
	prepared_changes: MessageReader<'w, 's, AssetEvent<PreparedModuleAsset<C>>>,
}

#[cfg(feature = "manifest")]
pub(crate) fn reconcile<C: FluentCatalog, M: LoadingMode>(
	mut localization: ResMut<Localization<C, M>>,
	source: Res<CatalogSource<C>>,
	mut assets: CatalogAssets<C>,
	mut updates: MessageWriter<CatalogUpdate<C>>,
) {
	if localization.has_dropped_leases() {
		localization.release_dropped_leases();
	}

	if localization.requests_changed && localization.preparation.is_some() {
		localization.synchronize_preparation_requests();
	}

	// Older Bevy versions mark Assets changed while tracking handles on idle
	// frames. Events identify real changes; pending reads are also polled so a
	// completion can publish before Bevy flushes its asset-event queue.
	let assets_changed = assets.changes.read().count() + assets.prepared_changes.read().count() > 0;

	let needs_reconcile = |state: &Localization<C, M>| {
		state.requests_changed || !state.retry.is_empty() || state.pending > 0 || assets_changed
	};
	let target_needs_reconcile = localization
		.preparation
		.as_ref()
		.is_some_and(|preparation| {
			preparation.locale() != localization.locale() && needs_reconcile(preparation)
		});

	if !needs_reconcile(&localization) && !target_needs_reconcile {
		return;
	}

	reconcile_state(
		&mut localization,
		&source,
		&assets.server,
		&assets.loaded,
		&assets.prepared,
		assets_changed,
		&mut updates,
	);

	if localization
		.prepared_locale()
		.is_some_and(|locale| locale != localization.locale())
		&& let Some(preparation) = localization.preparation.as_mut()
	{
		reconcile_state(
			preparation,
			&source,
			&assets.server,
			&assets.loaded,
			&assets.prepared,
			assets_changed,
			&mut updates,
		);
	}
}

#[cfg(feature = "manifest")]
fn reconcile_state<C: FluentCatalog, M: LoadingMode>(
	localization: &mut Localization<C, M>,
	source: &CatalogSource<C>,
	server: &AssetServer,
	assets: &Assets<ModuleAsset<C>>,
	prepared_assets: &Assets<PreparedModuleAsset<C>>,
	assets_changed: bool,
	updates: &mut MessageWriter<CatalogUpdate<C>>,
) {
	if !localization.requests_changed
		&& localization.retry.is_empty()
		&& localization.pending == 0
		&& !assets_changed
	{
		return;
	}

	if localization.requests_changed {
		localization.requests_changed = false;
	}

	release_unrequested(localization);
	let desired = localization.desired();

	let locale = localization.locale();

	for path in desired.iter() {
		let Some(module) = source.modules.get(path) else {
			continue;
		};
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
				localization
					.entries
					.insert(path, RequestedModule::new(locale));
				publish(localization, path, Err(error.clone()), updates);
				continue;
			}

			if source.manifest.embedded_modules().is_some() {
				localization
					.entries
					.insert(path, RequestedModule::new(locale));
				let candidate = source
					.manifest
					.read(locale.as_ref(), path)
					.map_err(|error| error.to_string())
					.and_then(|bytes| (module.parse)(locale, &bytes));
				publish(localization, path, candidate, updates);
				continue;
			}

			match asset_address(&source.manifest, locale.as_ref(), path) {
				Ok(address) => {
					// Settings are immutable for a living Bevy path handle. Retire
					// the old target handle before allocating a fresh attempt, so
					// canceled/late work cannot acquire a replacement's identity.
					if localization.staged {
						localization.entries.remove(path);

						if server.get_path_ids(address.clone()).iter().any(|id| {
							id.type_id() == std::any::TypeId::of::<PreparedModuleAsset<C>>()
						}) {
							localization.requests_changed = true;
							continue;
						}
					}

					let preparation_request = localization
						.staged
						.then(crate::preparation::next_asset_request);
					let (handle, preparation_handle, preparation_attempt) =
						if let Some(request) = preparation_request {
							let (handle, attempt) = crate::preparation::load_asset::<C>(
								server,
								address,
								request,
								&source.attempts,
							);
							(None, Some(handle), Some(attempt))
						} else if let Some(handle) = localization
							.entries
							.get(path)
							.and_then(|entry| entry.handle.clone())
						{
							// Reuse the active handle; reload requests are serialized per leaf.
							server.reload(address);
							(Some(handle), None, None)
						} else {
							(Some(server.load(address)), None, None)
						};
					let accepted = localization
						.entries
						.get(path)
						.and_then(|entry| entry.accepted);
					localization.pending += 1;
					localization.entries.insert(
						path,
						RequestedModule {
							handle,
							accepted,
							preparation_request,
							preparation_handle,
							preparation_attempt,
							pending: true,
							..RequestedModule::new(locale)
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
							..RequestedModule::new(locale)
						},
					);
					publish(localization, path, Err(error), updates);
				}
			}
		}

		let Some(entry) = localization.entries.get(path) else {
			continue;
		};
		let asset = if localization.staged {
			entry
				.preparation_handle
				.as_ref()
				.and_then(|handle| prepared_assets.get(handle))
				.map(|asset| (&asset.asset, Some(asset.request)))
		} else {
			entry
				.handle
				.as_ref()
				.and_then(|handle| assets.get(handle))
				.map(|asset| (asset, None))
		};
		let Some((asset, request)) = asset else {
			if localization.staged
				&& entry.pending
				&& entry
					.preparation_attempt
					.as_ref()
					.is_some_and(|attempt| attempt.finished_without_loader())
			{
				localization.finish_request(path);
				publish(localization, path, Err("locale preparation completed without the expected catalog loader; check source acquisition and asset metadata".into()), updates);
			}

			continue;
		};

		// Revisions order loader invocations, not completion. External overlapping
		// reads delayed before AssetLoader::load have no public Bevy request identity.
		if (localization.staged && entry.preparation_request != request)
			|| asset.locale != locale
			|| asset.path != path
			|| entry
				.accepted
				.is_some_and(|revision| asset.revision <= revision)
		{
			continue;
		}

		let entry = localization.finish_request(path);
		entry.accepted = Some(asset.revision);
		publish(localization, path, asset.candidate.clone(), updates);
	}

	if localization.staged {
		crate::preparation::refresh_handoff(localization, source, server);
	}
}

pub(crate) fn release_unrequested<C: FluentCatalog, M: LoadingMode>(
	localization: &mut Localization<C, M>,
) {
	let desired = localization.desired();
	let removed: std::collections::BTreeSet<_> = localization
		.entries
		.keys()
		.chain(localization.store.states.keys())
		.chain(localization.retry.iter())
		.copied()
		.filter(|path| !desired.contains(path))
		.collect();

	#[cfg(feature = "manifest")]
	localization
		.handoff_pending
		.retain(|path| desired.contains(path));

	for path in removed {
		if localization
			.entries
			.remove(path)
			.is_some_and(|entry| entry.pending)
		{
			localization.pending -= 1;
		}

		localization.retry.remove(path);
		localization.store.states.remove(path);

		if let Some(id) = localization.store.leaves.get(path).copied() {
			localization.store.values.remove(&id);
			localization.store.revision += 1;
		}
	}
}

pub(crate) fn publish<C: FluentCatalog, M: LoadingMode>(
	localization: &mut Localization<C, M>,
	path: &'static str,
	candidate: Result<crate::catalog::SharedScope, String>,
	updates: &mut MessageWriter<CatalogUpdate<C>>,
) {
	let locale = localization.locale();

	match candidate {
		Ok(value) => {
			localization.store.insert_leaf(path, value);

			if localization.staged {
				return;
			}

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

			if localization.staged {
				return;
			}

			updates.write(CatalogUpdate::Rejected {
				locale: Some(locale),
				path: path.into(),
				error,
			});
		}
	}
}

#[cfg(feature = "manifest")]
pub(crate) fn report_failures<C: FluentCatalog, M: LoadingMode>(
	mut events: MessageReader<AssetLoadFailedEvent<ModuleAsset<C>>>,
	mut prepared_events: MessageReader<AssetLoadFailedEvent<PreparedModuleAsset<C>>>,
	mut localization: ResMut<Localization<C, M>>,
	mut updates: MessageWriter<CatalogUpdate<C>>,
) {
	if localization.has_dropped_leases() {
		localization.release_dropped_leases();
		localization.synchronize_preparation_requests();
	}

	for event in events.read() {
		report_failure(&mut localization, event, &mut updates);
	}

	for event in prepared_events.read() {
		let Some(preparation) = localization.preparation.as_mut() else {
			continue;
		};
		let desired = preparation.desired();
		let path = preparation.entries.iter().find_map(|(path, entry)| {
			entry
				.preparation_handle
				.as_ref()
				.filter(|handle| desired.contains(path) && handle.id() == event.id)
				.map(|_| *path)
		});

		if let Some(path) = path {
			preparation.finish_request(path);
			publish(
				preparation,
				path,
				Err(event.error.to_string()),
				&mut updates,
			);
		}
	}
}

#[cfg(feature = "manifest")]
fn report_failure<C: FluentCatalog, M: LoadingMode>(
	localization: &mut Localization<C, M>,
	event: &AssetLoadFailedEvent<ModuleAsset<C>>,
	updates: &mut MessageWriter<CatalogUpdate<C>>,
) {
	let desired = localization.desired();
	let path = localization.entries.iter().find_map(|(path, entry)| {
		entry
			.handle
			.as_ref()
			.filter(|handle| desired.contains(path) && handle.id() == event.id)
			.map(|_| *path)
	});

	if let Some(path) = path {
		localization.finish_request(path);
		publish(localization, path, Err(event.error.to_string()), updates);
	}
}

#[cfg(feature = "manifest")]
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
