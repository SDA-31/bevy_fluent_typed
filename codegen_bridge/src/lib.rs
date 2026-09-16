//! Optional generated-code adapter, maintained with `bevy_fluent_typed`.
//!
//! Enable `build` only in build-dependencies and return `build()` from build.rs.
//! The main runtime's opt-in `codegen` feature enables this package's `runtime`
//! feature and exposes `bevy_fluent_typed::translations!`. Cargo resolver 2/3 separates contexts:
//! build-only use compiles no Bevy, runtime-only use compiles no generator.
//! Default features are empty. This bridge never depends on the Bevy runtime;
//! its macros receive the runtime path from the facade. The generator is independent.
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
