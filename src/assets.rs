//! One typed asset per logical module and locale, using Bevy's own source I/O.
mod preparation;

use crate::bevy::{
	asset::{self as bevy_asset, AssetLoader, LoadContext, io::Reader},
	prelude::*,
};
use crate::catalog::SharedScope;
use crate::{FluentCatalog, Module, compatibility};
pub(crate) use preparation::{
	AssetAttempt, PreparationGuard, PreparedModuleAsset, PreparedModuleLoader,
};
use std::{
	any::type_name,
	collections::HashMap,
	io,
	sync::atomic::{AtomicU64, Ordering},
};

static NEXT_REVISION: AtomicU64 = AtomicU64::new(1);

#[derive(Asset)]
pub(crate) struct ModuleAsset<C: FluentCatalog> {
	pub(crate) locale: C::Locale,
	pub(crate) path: &'static str,
	pub(crate) revision: u64,
	pub(crate) candidate: Result<SharedScope, String>,
}

impl<C: FluentCatalog> TypePath for ModuleAsset<C> {
	fn type_path() -> &'static str {
		type_name::<Self>()
	}

	fn short_type_path() -> &'static str {
		type_name::<Self>()
	}
}

pub(crate) struct ModuleLoader<C: FluentCatalog> {
	modules: Vec<Module<C>>,
	addresses: HashMap<crate::bevy::asset::AssetPath<'static>, (C::Locale, usize)>,
}

impl<C: FluentCatalog> ModuleLoader<C> {
	pub(crate) fn new(manifest: &crate::LocalizationManifest) -> Self {
		let modules = C::modules();
		let mut addresses = HashMap::new();

		if manifest.file_path().is_some() {
			for &locale in C::locales() {
				for (index, module) in modules.iter().enumerate() {
					if let Ok(address) =
						crate::loading::asset_address(manifest, locale.as_ref(), module.path)
					{
						addresses.insert(address, (locale, index));
					}
				}
			}
		}

		Self { modules, addresses }
	}
}

impl<C: FluentCatalog> TypePath for ModuleLoader<C> {
	fn type_path() -> &'static str {
		type_name::<Self>()
	}

	fn short_type_path() -> &'static str {
		type_name::<Self>()
	}
}

impl<C: FluentCatalog> AssetLoader for ModuleLoader<C> {
	type Asset = ModuleAsset<C>;
	type Settings = ();
	type Error = io::Error;

	async fn load(
		&self,
		reader: &mut dyn Reader,
		_: &(),
		context: &mut LoadContext<'_>,
	) -> Result<Self::Asset, Self::Error> {
		let revision = NEXT_REVISION.fetch_add(1, Ordering::Relaxed);
		let address = compatibility::asset_path(context);
		let (locale, index) = self.addresses.get(address).ok_or_else(|| {
			io::Error::other("asset address is absent from this provider's source contract")
		})?;
		let locale = *locale;
		let module = &self.modules[*index];
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		Ok(ModuleAsset {
			locale,
			path: module.path,
			revision,
			candidate: (module.parse)(locale, &bytes),
		})
	}
}
