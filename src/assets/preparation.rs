//! Target catalogs use a private typed asset namespace and the normal checked loader.
use super::{ModuleAsset, ModuleLoader};
use crate::FluentCatalog;
use crate::bevy::{
	asset::{self as bevy_asset, AssetLoader, LoadContext, io::Reader},
	prelude::*,
};
use std::{
	any::type_name,
	io,
	sync::{
		Arc,
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

pub(crate) struct PreparedModuleLoader<C: FluentCatalog>(pub(crate) ModuleLoader<C>);

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
		let asset = self.0.load(reader, &(), context).await?;
		Ok(PreparedModuleAsset {
			asset,
			request: request.0,
		})
	}
}

/// A completed task with incompatible settings cannot ever produce our asset.
/// This also covers Bevy's early metadata/type-mismatch return without an event.
#[derive(Default)]
pub(crate) struct AssetAttempt {
	pub(crate) settings_applied: AtomicBool,
	finished: AtomicBool,
}

impl AssetAttempt {
	pub(crate) fn finished_without_settings(&self) -> bool {
		self.finished.load(Ordering::Acquire) && !self.settings_applied.load(Ordering::Acquire)
	}
}

pub(crate) struct PreparationGuard(pub(crate) Arc<AssetAttempt>);

impl Drop for PreparationGuard {
	fn drop(&mut self) {
		self.0.finished.store(true, Ordering::Release);
	}
}
