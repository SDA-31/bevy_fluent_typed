//! Demand-driven byte acquisition, independent of asset addresses and TOML.
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::*,
	tasks::{IoTaskPool, futures_lite::future},
};
use crate::catalog::SharedScope;
use crate::compatibility::MessageWriter;
use crate::loading::{publish, release_unrequested};
use crate::state::RequestedModule;
use crate::{CatalogUpdate, FluentCatalog, LoadingMode, Localization, Module, ModuleStatus};
use std::{collections::HashMap, fmt::Display, future::Future, pin::Pin, sync::Arc};

type LoadFuture = Pin<Box<dyn Future<Output = Result<SharedScope, String>> + Send>>;

type Load<C> = dyn Fn(<C as FluentCatalog>::Locale, &Module<C>) -> LoadFuture + Send + Sync;

#[derive(Clone)]
pub(crate) struct ByteLoader<C: FluentCatalog>(Arc<Load<C>>);

impl<C: FluentCatalog> ByteLoader<C> {
	pub(crate) fn new<L, F, B, E>(load: L) -> Self
	where
		L: Fn(C::Locale, &'static str) -> F + Send + Sync + 'static,
		F: Future<Output = Result<B, E>> + Send + 'static,
		B: AsRef<[u8]> + Send + 'static,
		E: Display,
	{
		let load = Arc::new(load);

		Self(Arc::new(move |locale, module| {
			let load = load.clone();
			let parse = module.parse.clone();
			let path = module.path;

			Box::pin(async move {
				let bytes = load(locale, path)
					.await
					.map_err(|error| format!("load {}/{path}: {error}", locale.as_ref()))?;
				parse(locale, bytes.as_ref())
					.map_err(|error| format!("parse {}/{path}: {error}", locale.as_ref()))
			})
		}))
	}
}

#[derive(Resource)]
pub(crate) struct ByteSource<C: FluentCatalog> {
	pub(crate) loader: ByteLoader<C>,
	pub(crate) modules: HashMap<&'static str, Module<C>>,
}

pub(crate) fn reconcile<C: FluentCatalog, M: LoadingMode>(
	mut localization: ResMut<Localization<C, M>>,
	source: Res<ByteSource<C>>,
	mut updates: MessageWriter<CatalogUpdate<C>>,
) {
	if !localization.requests_changed && localization.retry.is_empty() && localization.pending == 0
	{
		return;
	}

	localization.requests_changed = false;
	release_unrequested(&mut localization);
	let locale = localization.locale();

	for &path in localization.desired().iter() {
		let Some(module) = source.modules.get(path) else {
			continue;
		};
		let pending = localization
			.entries
			.get(path)
			.is_some_and(|entry| entry.pending);
		let retry = localization.retry.contains(path) && !pending;
		let initial = !localization.entries.contains_key(path);

		if initial || retry {
			localization.retry.remove(path);
			localization
				.store
				.states
				.insert(path, ModuleStatus::Loading);
			let mut entry = RequestedModule::new(locale);
			entry.task = Some(IoTaskPool::get().spawn((source.loader.0)(locale, module)));
			entry.pending = true;
			localization.entries.insert(path, entry);
			localization.pending += 1;
		}

		let Some(entry) = localization.entries.get_mut(path) else {
			continue;
		};
		let Some(task) = entry.task.as_mut() else {
			continue;
		};
		let Some(candidate) = future::block_on(future::poll_once(task)) else {
			continue;
		};

		// Completion belongs to this entry. Removing it on unload or locale change
		// makes an old result unreachable, even if external I/O keeps running.
		debug_assert!(entry.locale == locale);
		localization.finish_request(path).task = None;
		publish(&mut localization, path, candidate, &mut updates);
	}
}
