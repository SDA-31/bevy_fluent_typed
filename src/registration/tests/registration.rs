use crate::bevy::prelude::{App, MinimalPlugins};
use crate::tests::TestCatalog;
use crate::{CachePolicy, Localization, LocalizationPlugin, LocalizationRegistrationError, Manual};
use std::{
	any::{TypeId, type_name},
	panic::{AssertUnwindSafe, catch_unwind},
};

#[test]
fn duplicate_plugin_reports_the_catalog_and_conflicting_modules() {
	let first = LocalizationPlugin::<TestCatalog>::from_bytes::<&[u8]>([]).unwrap();
	let second = LocalizationPlugin::<TestCatalog>::from_loader(
		|_, _| -> std::future::Ready<Result<Vec<u8>, String>> {
			panic!("registration must not invoke the loader");
		},
	);
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, first));

	let panic = catch_unwind(AssertUnwindSafe(|| {
		app.add_plugins(second);
	}))
	.unwrap_err();
	let message = panic.downcast_ref::<String>().unwrap();
	assert!(message.contains("conflicting localization plugin registrations"));
	assert!(message.contains(type_name::<TestCatalog>()));
	assert!(message.contains("ui.ftl"));
}

#[test]
fn preflight_returns_a_typed_conflict_and_preserves_the_first_provider() {
	let first = LocalizationPlugin::<TestCatalog>::from_bytes::<&[u8]>([]).unwrap();
	let second = LocalizationPlugin::<TestCatalog, Manual>::from_loader(|_, _| {
		std::future::pending::<Result<Vec<u8>, String>>()
	})
	.with_cache_policy(CachePolicy::None);
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, first));
	let error: LocalizationRegistrationError = second.validate_registration(&app).unwrap_err();
	assert_eq!(error.catalog, TypeId::of::<TestCatalog>());
	assert_eq!(error.catalog_name, type_name::<TestCatalog>());
	assert_eq!(error.modules, ["ui.ftl"]);
	assert_eq!(
		error.first_plugin,
		type_name::<LocalizationPlugin<TestCatalog>>()
	);
	assert_eq!(
		error.second_plugin,
		type_name::<LocalizationPlugin<TestCatalog, Manual>>()
	);
	assert_eq!(error.first_created_at.file(), file!());
	assert_eq!(error.second_created_at.file(), file!());
	assert_ne!(
		error.first_created_at.line(),
		error.second_created_at.line()
	);
	assert_eq!(error, second.validate_registration(&app).unwrap_err());
	let _: &dyn std::error::Error = &error;

	let panic = catch_unwind(AssertUnwindSafe(|| {
		app.add_plugins(second);
	}))
	.unwrap_err();
	assert_eq!(panic.downcast_ref::<String>().unwrap(), &error.to_string());
	assert!(app.world().contains_resource::<Localization<TestCatalog>>());
	assert!(
		!app.world()
			.contains_resource::<Localization<TestCatalog, Manual>>()
	);
	assert_eq!(
		app.world()
			.resource::<crate::cache_policy::ProviderCachePolicy<TestCatalog>>()
			.policy,
		CachePolicy::Rendered
	);
}

#[test]
fn preflight_without_an_existing_provider_does_not_install_or_reserve_anything() {
	let plugin = LocalizationPlugin::<TestCatalog>::from_loader(|_, _| {
		std::future::pending::<Result<Vec<u8>, String>>()
	});
	let mut app = App::new();
	plugin.validate_registration(&app).unwrap();
	plugin.validate_registration(&app).unwrap();
	assert!(
		!app.world()
			.contains_resource::<crate::progress::Registration<TestCatalog>>()
	);
	assert!(!app.world().contains_resource::<Localization<TestCatalog>>());

	app.add_plugins((MinimalPlugins, plugin));
	assert!(app.world().contains_resource::<Localization<TestCatalog>>());
}

#[cfg(feature = "manifest")]
#[derive(Clone, Copy)]
enum SourceKind {
	File,
	Embedded,
	Bytes,
	Loader,
}

#[cfg(feature = "manifest")]
#[track_caller]
fn source(
	kind: SourceKind,
	reads: std::sync::Arc<std::sync::atomic::AtomicUsize>,
) -> LocalizationPlugin<TestCatalog> {
	use crate::{CatalogConfig, LocalizationManifest};

	match kind {
		SourceKind::File => LocalizationPlugin::new(LocalizationManifest::from_config(
			CatalogConfig {
				source_language: "ja".into(),
				default_language: "ja".into(),
				languages_directory: "missing-translations".into(),
			},
			"missing-localization-contract.toml",
		)),
		SourceKind::Embedded => LocalizationPlugin::new(LocalizationManifest::__embedded((
			"ja",
			"ja",
			".",
			&[("es", "ui.ftl", b"embedded text")],
		))),
		SourceKind::Bytes => {
			LocalizationPlugin::from_bytes([("ja", "ui.ftl", b"different byte text".as_slice())])
				.unwrap()
		}
		SourceKind::Loader => LocalizationPlugin::from_loader(move |_, _| {
			reads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
			std::future::ready(Ok::<_, String>(b"loader text".to_vec()))
		}),
	}
}

#[cfg(feature = "manifest")]
#[test]
fn registration_conflicts_ignore_every_pair_of_source_kinds() {
	use crate::bevy::asset::AssetPlugin;
	use std::sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	};

	for first_kind in [
		SourceKind::File,
		SourceKind::Embedded,
		SourceKind::Bytes,
		SourceKind::Loader,
	] {
		for second_kind in [
			SourceKind::File,
			SourceKind::Embedded,
			SourceKind::Bytes,
			SourceKind::Loader,
		] {
			let reads = Arc::new(AtomicUsize::new(0));
			let first = source(first_kind, reads.clone());
			let second = source(second_kind, reads.clone());
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, AssetPlugin::default(), first));
			let error = second.validate_registration(&app).unwrap_err();
			assert_eq!(error.modules, ["ui.ftl"]);
			assert_eq!(error.first_created_at.file(), file!());
			assert_eq!(error.second_created_at.file(), file!());
			assert_ne!(
				error.first_created_at.line(),
				error.second_created_at.line()
			);

			let panic = catch_unwind(AssertUnwindSafe(|| {
				app.add_plugins(second);
			}))
			.unwrap_err();
			assert_eq!(panic.downcast_ref::<String>().unwrap(), &error.to_string());
			assert_eq!(reads.load(Ordering::Relaxed), 0);
		}
	}
}

#[cfg(feature = "manifest")]
#[test]
fn new_manual_preserves_its_constructor_call_site() {
	use crate::bevy::asset::AssetPlugin;

	let first = LocalizationPlugin::<TestCatalog>::new_manual(crate::tests::manifest());
	let second = LocalizationPlugin::<TestCatalog>::new(crate::tests::manifest());
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default(), first));
	let error = second.validate_registration(&app).unwrap_err();
	assert_eq!(
		error.first_plugin,
		type_name::<LocalizationPlugin<TestCatalog, Manual>>()
	);
	assert_eq!(error.first_created_at.file(), file!());
	assert_eq!(error.second_created_at.file(), file!());
	assert_ne!(
		error.first_created_at.line(),
		error.second_created_at.line()
	);
}
