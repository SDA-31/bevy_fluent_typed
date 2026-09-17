//! Connect [fluent_typed_codegen](https://docs.rs/fluent_typed_codegen/) output
//! to the [bevy_fluent_typed](https://docs.rs/bevy_fluent_typed/) runtime.
//! Message accessors and Fluent resolution are provided by
//! [fluent-typed](https://docs.rs/fluent-typed/).
//!
//! This optional companion generates the provider and shared module resources
//! consumed by the runtime's localization plugin. Discovery and typed Fluent
//! accessors belong to the generator; loading, language selection, hot reload and
//! UI updates belong to the runtime. The bridge supplies the adapter between them.
//!
//! # Features and setup
//!
//! - **`build`:** generation entrypoints and `Settings`. Enable only in
//!   the host graph. Consumers normally enable `bevy_fluent_typed/build` with
//!   defaults disabled and call `bevy_fluent_typed::build()` in `build.rs`.
//!   It emits the additional provider/resource tree into `OUT_DIR`.
//! - **`runtime`:** output inclusion and checked definition parsing. The runtime's
//!   `codegen` feature enables this and exposes its `translations!` facade.
//! - **No features (default):** no adapter code or dependencies.
//!
//! Declare `bevy_fluent_typed::translations!(pub mod texts)` in application source.
//! This includes generated output; it does not generate files or install a plugin.
//! The [complete setup and asset example](https://github.com/SDA-31/bevy_fluent_typed/blob/main/GUIDE.md)
//! shows configuration, a build script and runtime registration. The
//! [bridge README](https://github.com/SDA-31/bevy_fluent_typed/tree/main/codegen_bridge)
//! describes low-level entrypoints and feature boundaries. The public build
//! facade is available since 0.1.1; version 0.1.0 used the bridge directly.
//!
//! # Dependency and reload boundaries
//!
//! With Cargo resolver 2/3, build-only use compiles no Bevy and runtime-only use
//! compiles no generator. The bridge never depends on `bevy_fluent_typed`:
//! its macro receives the facade's runtime path, avoiding a dependency cycle.
//! The declared minimum Rust version is 1.95.
//! Engine backend selection belongs to the runtime, not this build dependency.
//! Generated declarations use its macro to preserve ECS-immutable resources on
//! Bevy 0.19 while supporting ordinary resources on 0.17/0.18.
//!
//! The generated provider checks complete catalogs and immutable configuration.
//! Compatible prose edits can reload; changes to configuration, languages, modules
//! or typed contracts require generation and restart. The bridge does not watch
//! files or publish resources itself: it emits the provider used by the runtime.
//! Native numeric selectors remain available. Number text and plural keywords
//! from [fluent_typed_decimal](https://docs.rs/fluent_typed_decimal/) pass through
//! ordinary String parameters; the bridge has no Decimal dependency or formatting
//! policy. RTL layout, glyph shaping and fonts belong to the application's renderer.
//! Repository links follow `main`; this reference describes the viewed version.
#![warn(missing_docs)]

#[cfg(feature = "build")]
mod generation;

#[cfg(feature = "build")]
mod provider;

#[cfg(feature = "runtime")]
mod definition;

#[cfg(feature = "runtime")]
mod macros;

#[cfg(feature = "build")]
pub use generation::{build, from_cargo, generate};

/// Generator settings for explicit build frontends.
#[cfg(feature = "build")]
pub use fluent_typed_codegen::Settings;

/// Parser used by the generated strict reload contracts.
#[doc(hidden)]
#[cfg(feature = "runtime")]
pub use fluent_syntax;

#[doc(hidden)]
#[cfg(feature = "runtime")]
pub use definition::validate_definition;
