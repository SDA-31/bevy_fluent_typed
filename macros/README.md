# Component adapter derive

Implementation of `bevy_fluent_typed::LocalizationTarget` derive. Applications use
the runtime facade with `codegen` (which includes `derive`) or enable `derive`
separately. No additional application plugin is required.

The derive selects one `#[localized]` String field on a mutable Default component
and generates an implementation of the runtime's public target contract. Manual
Default implementations, generic components and renamed runtime dependencies
(`#[localization(crate = runtime_alias)]`) are supported. Compound, immutable and
non-Default targets can implement the same runtime trait manually.

This procedural macro transforms typed Rust syntax. It does not discover
translation files or perform generation/file writes; translation generation
remains an explicit consumer `build.rs` call.

[MIT](LICENSE).
