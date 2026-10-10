/// Declare a typed translation tree prepared by this crate's build API.
///
/// Requires feature `codegen`. In build.rs, call
/// `bevy_fluent_typed::build()` using a build-dependency with defaults disabled
/// and feature `build`. This macro includes prepared output and resolves its
/// runtime path hygienically, including when the Cargo dependency is renamed.
/// Module attributes and Rust visibility (`pub`, `pub(crate)`, etc.) are supported.
/// With Cargo resolver 2/3, generation stays
/// in the build-script dependency graph rather than the application runtime.
/// Declaration alone embeds no FTL, including in unoptimized builds.
/// `texts::manifest()` keeps the build-configured `CATALOG_PATH` origin without I/O.
/// The application owns its engine source root and any different runtime origin.
/// Declare an embedded source with a selector relative to the generated tree:
/// `texts::embed_manifest! { pub const HUD = presentation::Hud; }`.
/// The constant has type [`crate::LocalizationManifest`] and can be exported from
/// a private localization module alongside a catalog alias such as `Hud as Interface`.
/// Select a leaf, a group such as `Presentation`, or the complete `Translations` tree.
/// Selectors use original schema names, independently of application imports.
/// Ordinary aliases still work for catalog constructors and `Res<Interface>`.
/// Only the empty invocation `texts::embed_manifest!()` for the complete tree
/// and the constant-declaration block are accepted.
///
/// ```ignore
/// bevy_fluent_typed::translations!(pub mod texts);
/// ```
#[macro_export]
macro_rules! translations {
	($(#[$attribute:meta])* $visibility:vis mod $name:ident $(;)?) => {
		/// Typed translation resources generated from the consuming package's catalogs.
		$(#[$attribute])*
		// Upstream emits optional helpers and parameter lists consumers may not use.
		#[allow(dead_code, clippy::derivable_impls, clippy::too_many_arguments)]
		$visibility mod $name {
			use $crate as __fluent_runtime;

			::std::include!(::std::concat!(::std::env!("OUT_DIR"), "/bevy_catalog.rs"));
		}
	};
}

/// Apply the selected engine's resource contract to a generated declaration.
/// Chosen in the runtime graph, never through the consumer's cfg or build.rs.
#[doc(hidden)]
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[macro_export]
macro_rules! __localized_resource {
	($declaration:item) => {
		#[derive($crate::bevy::prelude::Resource)]
		#[component(immutable)]
		$declaration
	};
}

/// Older Bevy backends have resources but no ECS-level resource immutability.
#[doc(hidden)]
#[cfg(not(any(feature = "bevy-0-19", feature = "bevy-0-20")))]
#[macro_export]
macro_rules! __localized_resource {
	($declaration:item) => {
		#[derive($crate::bevy::prelude::Resource)]
		$declaration
	};
}
