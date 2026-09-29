//! Public mode selection and scheduling for independently loaded scopes.
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{
	FluentCatalog, Full, LoadingMode, Localization,
	bytes::{self, ByteLoader, ByteSource},
	compatibility,
	loading::reload_catalogs,
	resources,
	systems::CatalogReadiness,
};
#[cfg(feature = "manifest")]
use crate::{
	Lazy, LocalizationManifest,
	assets::{ModuleAsset, ModuleLoader},
	loading::{CatalogSource, reconcile, report_failures},
};
use std::{
	collections::{HashMap, HashSet},
	fmt::Display,
	future::Future,
	marker::PhantomData,
	sync::Arc,
};

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

enum Source<C: FluentCatalog> {
	#[cfg(feature = "manifest")]
	Manifest(LocalizationManifest),
	Bytes(ByteLoader<C>),
}

/// Register loading and typed resources from a manifest or application-owned byte source.
///
/// Manifest sources require AssetPlugin; byte sources need only MinimalPlugins.
/// Full mode requests all modules of the selected locale;
/// Lazy mode waits for explicit requests. Creating this value performs no I/O.
/// Only one plugin/controller mode may own a given root provider in an App.
pub struct LocalizationPlugin<C: FluentCatalog, M: LoadingMode = Full> {
	source: Source<C>,
	marker: PhantomData<fn() -> (C, M)>,
}

impl<C: FluentCatalog, M: LoadingMode> LocalizationPlugin<C, M> {
	/// Configure the selected loading mode from a ready source contract.
	/// File origins are Bevy asset addresses; named sources are preserved.
	/// Generated consumers can pass `texts::manifest()` for their build-configured
	/// file source, or a `LocalizationManifest` constant declared by
	/// `texts::embed_manifest! { const SOURCE = Translations; }` for explicit embedding.
	/// Pass that constant directly to `new(SOURCE)`; no conversion is needed.
	#[cfg(feature = "manifest")]
	pub fn new(manifest: LocalizationManifest) -> Self {
		Self {
			source: Source::Manifest(manifest),
			marker: PhantomData,
		}
	}

	/// Retain readable module bytes, parsing only the modules requested by Full/Lazy.
	/// Each tuple contains a compiled locale, a logical leaf path (e.g. `Hud::PATH`)
	/// and owned bytes. The provider's default locale is selected initially.
	/// Missing modules fail when requested. Retained buffers survive `unload`;
	/// use [`Self::from_loader`] to retrieve and release source buffers on demand.
	///
	/// # Errors
	/// Rejects unknown locales, unknown leaf paths and duplicate `(locale, path)` keys.
	pub fn from_bytes<B>(
		modules: impl IntoIterator<Item = (C::Locale, &'static str, B)>,
	) -> Result<Self, String>
	where
		B: Into<Arc<[u8]>>,
	{
		let known: HashSet<_> = C::modules().into_iter().map(|module| module.path).collect();
		let mut sources = HashMap::new();

		for (locale, path, bytes) in modules {
			if !C::locales().contains(&locale) || !known.contains(path) {
				return Err(format!("unknown module {}/{path}", locale.as_ref()));
			}

			if sources
				.insert((locale.as_ref().to_owned(), path), bytes.into())
				.is_some()
			{
				return Err(format!("duplicate module {}/{path}", locale.as_ref()));
			}
		}

		Ok(Self::from_loader(move |locale, path| {
			let bytes = sources
				.get(&(locale.as_ref().to_owned(), path))
				.cloned()
				.ok_or_else(|| format!("missing module {}/{path}", locale.as_ref()));
			std::future::ready(bytes)
		}))
	}

	/// Retrieve readable bytes asynchronously for each requested locale and leaf.
	/// Works with generated or handwritten providers, without a runtime manifest.
	/// The provider's default locale is selected initially; a preinserted
	/// [`Localization<C, M>`] may select another compiled locale.
	///
	/// The callback and checked parser run on Bevy's I/O task pool. Return owned
	/// bytes (such as `Vec<u8>`); the temporary input is dropped after parsing.
	/// Callback futures must be nonblocking: single-threaded backends may poll
	/// them on the main thread. An async function does not make blocking I/O safe.
	/// Futures requiring a specific executor must be integrated by the application.
	///
	/// Unload and locale changes discard obsolete results. External I/O may continue;
	/// older Bevy wasm task pools do not cancel the underlying future on task drop.
	/// Requests are serialized per active leaf, retries coalesce, and a failed
	/// reload retains the last good same-locale snapshot. There is no source cache,
	/// automatic retry or fallback. Handwritten providers own their parser checks.
	pub fn from_loader<L, F, B, E>(load: L) -> Self
	where
		L: Fn(C::Locale, &'static str) -> F + Send + Sync + 'static,
		F: Future<Output = Result<B, E>> + Send + 'static,
		B: AsRef<[u8]> + Send + 'static,
		E: Display,
	{
		Self {
			source: Source::Bytes(ByteLoader::new(load)),
			marker: PhantomData,
		}
	}
}

#[cfg(feature = "manifest")]
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
		let startup = match &self.source {
			#[cfg(feature = "manifest")]
			Source::Manifest(manifest) => C::locales()
				.iter()
				.copied()
				.find(|locale| locale.as_ref() == manifest.config().default_language),
			Source::Bytes(_) => Some(C::default_locale()),
		};

		if !app.world().contains_resource::<Localization<C, M>>() {
			app.insert_resource(Localization::<C, M>::new(
				startup.unwrap_or_else(C::default_locale),
			));
		}

		app.insert_resource(Installed::<C>(PhantomData))
			.init_resource::<Localization<C, M>>();
		compatibility::register_notifications::<C>(app);

		for scope in C::scopes() {
			CatalogReadiness::register::<C, M>(app.world_mut(), &scope);
			(scope.bindings)(app);
		}

		match &self.source {
			Source::Bytes(loader) => {
				app.insert_resource(ByteSource::<C> {
					loader: loader.clone(),
					modules: C::modules()
						.into_iter()
						.map(|module| (module.path, module))
						.collect(),
				})
				.add_systems(
					PreUpdate,
					(
						reload_catalogs::<C, M>,
						bytes::reconcile::<C, M>,
						resources::synchronize::<C, M>,
					)
						.chain()
						.in_set(LocalizationSystems::Publish),
				)
				.add_systems(
					PostUpdate,
					(bytes::reconcile::<C, M>, resources::synchronize::<C, M>)
						.chain()
						.before(LocalizationSystems::Refresh),
				);
			}
			#[cfg(feature = "manifest")]
			Source::Manifest(manifest) => {
				let error = if manifest.config().source_language != C::source_locale().as_ref() {
					Some("manifest source-language differs from the compiled schema".into())
				} else if startup.is_none() {
					Some("manifest default-language is not a compiled locale".into())
				} else {
					None
				};

				app.insert_resource(CatalogSource::<C> {
					manifest: manifest.clone(),
					modules: C::modules()
						.into_iter()
						.map(|module| (module.path, module))
						.collect(),
					error,
					marker: PhantomData,
				})
				.init_asset::<ModuleAsset<C>>()
				.register_asset_loader(ModuleLoader::<C>::new(manifest));

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
	}
}
