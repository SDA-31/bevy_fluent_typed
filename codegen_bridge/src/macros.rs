/// Declare a module containing this application's generated translations.
///
/// Run `bevy_fluent_codegen_bridge::build()` from the consuming package's `build.rs`
/// with the bridge's `build` feature enabled first. This macro includes that package's
/// Cargo output; it does not generate files or register the Bevy plugin itself.
/// No handwritten `generated.rs`, output path or runtime dependency alias is needed.
///
/// The module contains `Translations`, `Locale`, named groups/leaves and asset
/// metadata. Usually use the main runtime's `translations!` facade instead.
/// Module attributes and Rust visibility (`pub`, `pub(crate)`, etc.) are supported.
///
/// ```ignore
/// bevy_fluent_codegen_bridge::translations!(runtime = bevy_fluent_typed; pub mod texts);
///
/// use texts::{Locale, Translations, presentation};
/// ```
///
/// The example requires application-owned FTL sources and build output, so it is
/// compiled in `examples/minimal` rather than as a standalone doctest. With a
/// renamed Cargo dependency, call `your_dependency_name::translations!` instead.
/// Generated types and validation are unchanged; their sources stay in Cargo's
/// target directory. Rebuild after schema changes; compatible text edits can
/// still hot reload through the runtime plugin.
#[macro_export]
macro_rules! translations {
	(runtime = $runtime:path; $(#[$attribute:meta])* $visibility:vis mod $name:ident $(;)?) => {
		/// Typed translation resources generated from the consuming package's catalogs.
		$(#[$attribute])*
		// Upstream emits optional helpers and parameter lists consumers may not use.
		#[allow(dead_code, clippy::derivable_impls, clippy::too_many_arguments)]
		$visibility mod $name {
			use $crate as __fluent_bridge;
			use $runtime as __fluent_runtime;

			::std::include!(::std::concat!(::std::env!("OUT_DIR"), "/bevy_catalog.rs"));
		}
	};
}
