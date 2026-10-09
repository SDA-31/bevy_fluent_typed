//! Target catalogs use a private typed asset namespace and the normal checked loader.
use super::{ModuleAsset, ModuleLoader};
use crate::FluentCatalog;
use crate::bevy::{
	asset::{self as bevy_asset, AssetLoader, LoadContext, io::Reader},
	prelude::*,
};
use std::{
	any::type_name,
	collections::BTreeMap,
	io,
	marker::PhantomData,
	sync::{
		Arc, Mutex, Weak,
		atomic::{AtomicBool, Ordering},
	},
};

/// A separate asset namespace isolates target work from published file assets.
#[derive(Asset)]
pub(crate) struct PreparedModuleAsset<C: FluentCatalog> {
	pub(crate) asset: ModuleAsset<C>,
	pub(crate) request: u64,
}

impl<C: FluentCatalog> TypePath for PreparedModuleAsset<C> {
	fn type_path() -> &'static str {
		type_name::<Self>()
	}

	fn short_type_path() -> &'static str {
		type_name::<Self>()
	}
}

pub(crate) struct PreparedModuleLoader<C: FluentCatalog> {
	pub(crate) loader: ModuleLoader<C>,
	pub(crate) attempts: PreparationAttempts<C>,
}

impl<C: FluentCatalog> TypePath for PreparedModuleLoader<C> {
	fn type_path() -> &'static str {
		type_name::<Self>()
	}

	fn short_type_path() -> &'static str {
		type_name::<Self>()
	}
}

impl<C: FluentCatalog> AssetLoader for PreparedModuleLoader<C> {
	type Asset = PreparedModuleAsset<C>;
	type Settings = (u64, ());
	type Error = io::Error;

	async fn load(
		&self,
		reader: &mut dyn Reader,
		request: &(u64, ()),
		context: &mut LoadContext<'_>,
	) -> Result<Self::Asset, Self::Error> {
		self.attempts.mark_started(request.0);

		let asset = self.loader.load(reader, &(), context).await?;
		Ok(PreparedModuleAsset {
			asset,
			request: request.0,
		})
	}
}

type AttemptMap = Mutex<BTreeMap<u64, Weak<AssetAttempt>>>;

/// Each installed catalog owns a separate registry; settings shape is not identity.
#[derive(Clone)]
pub(crate) struct PreparationAttempts<C: FluentCatalog> {
	attempts: Arc<AttemptMap>,
	marker: PhantomData<fn() -> C>,
}

impl<C: FluentCatalog> Default for PreparationAttempts<C> {
	fn default() -> Self {
		Self {
			attempts: Arc::default(),
			marker: PhantomData,
		}
	}
}

impl<C: FluentCatalog> PreparationAttempts<C> {
	pub(crate) fn register(&self, request: u64) -> Arc<AssetAttempt> {
		let attempt = Arc::new(AssetAttempt {
			settings_applied: AtomicBool::new(false),
			started: AtomicBool::new(false),
			finished: AtomicBool::new(false),
			request,
			registry: Arc::downgrade(&self.attempts),
		});
		self.attempts
			.lock()
			.expect("preparation attempt registry")
			.insert(request, Arc::downgrade(&attempt));
		attempt
	}

	fn mark_started(&self, request: u64) {
		let attempt = self
			.attempts
			.lock()
			.expect("preparation attempt registry")
			.get(&request)
			.and_then(Weak::upgrade);

		if let Some(attempt) = attempt {
			attempt.started.store(true, Ordering::Release);
		}
	}

	#[cfg(test)]
	pub(crate) fn len(&self) -> usize {
		self.attempts
			.lock()
			.expect("preparation attempt registry")
			.len()
	}
}

/// A completed task which never entered this catalog's loader cannot publish its asset.
pub(crate) struct AssetAttempt {
	pub(crate) settings_applied: AtomicBool,
	started: AtomicBool,
	finished: AtomicBool,
	request: u64,
	registry: Weak<AttemptMap>,
}

impl AssetAttempt {
	pub(crate) fn finished_without_loader(&self) -> bool {
		self.finished.load(Ordering::Acquire) && !self.started.load(Ordering::Acquire)
	}
}

impl Drop for AssetAttempt {
	fn drop(&mut self) {
		if let Some(registry) = self.registry.upgrade() {
			registry
				.lock()
				.expect("preparation attempt registry")
				.remove(&self.request);
		}
	}
}

pub(crate) struct PreparationGuard(pub(crate) Weak<AssetAttempt>);

impl Drop for PreparationGuard {
	fn drop(&mut self) {
		if let Some(attempt) = self.0.upgrade() {
			attempt.finished.store(true, Ordering::Release);
		}
	}
}
