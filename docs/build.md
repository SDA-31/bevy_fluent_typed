## Explicit build-script API

For a complete Cargo.toml, files and application, follow the
[quickstart](https://github.com/SDA-31/bevy_fluent_typed/blob/main/README.md#setup).
This section explains the optional custom build entrypoints.

The `build` feature exposes `build()`, `from_cargo()`, `generate()` and
`Settings` and `BuildError` through the same crate used by the application. Its private build
module emits the Bevy provider; no separate bridge package is needed.
Discovery and the typed module tree come from
[fluent_typed_codegen](https://docs.rs/fluent_typed_codegen/); message accessors
and Fluent resolution come from [fluent-typed](https://docs.rs/fluent-typed/).

In build-dependencies, disable default features and enable **only `build`**.
This configuration compiles the generator, not Bevy or the localization runtime.
Normal dependencies enable a Bevy backend and `codegen`, not `build`. Use Cargo
resolver 2 or 3 so host and target features remain separate.

```no_run
fn main() -> std::process::ExitCode {
    bevy_fluent_typed::build()
}
```

Generation and Cargo file/directory tracking run exclusively in this explicit
call. The application's `translations!` macro only includes the prepared output
under `OUT_DIR`; macro expansion never runs the generator or writes files.

For custom diagnostics use `from_cargo()` and handle its error; for explicit
package/output paths use `generate()` with `Settings`. Do not ignore generation
failures or compile stale output after an error.

```no_run
fn main() -> Result<(), bevy_fluent_typed::BuildError> {
    bevy_fluent_typed::from_cargo()
}
```

Both `from_cargo()` and `generate()` return the generator's `BuildError`,
re-exported by this facade. Match `BuildError::Io` for the operation, filesystem
path and original I/O error; `BuildError::Config` preserves the typed configuration
cause and available filename. Module differences expose sorted missing/extra
paths, and upstream failures keep their original typed error. All variants print
readable diagnostics with `Display` and expose their original causes through
`std::error::Error::source()` where applicable.

`Settings::from_manifest` and `CatalogConfig::parse` return `ConfigError`;
`LocalizationManifest` wraps configuration failures in `ManifestError::Config`.
Use `ConfigField`, `FieldError` and `PathError` for matching invalid known fields
or paths; unknown configuration fields remain ignored. Runtime-only consumers
without `manifest` or `codegen` still have no generator dependency. Build-only
errors are available only with `build`; they do not enable an engine backend.

Cargo feature forwarding such as `bevy_fluent_typed/bevy-0-17` can enable a feature
on both dependency kinds when they share a name. Select the backend directly in
the **normal dependency's** `features` array, not through application feature
forwarding. Cargo also requires the same dependency name in both sections; if
renaming the crate, use that alias in both, as in the integration example.

The public facade is available since **0.1.1**. Version 0.1.0 required a separate
bridge build-dependency. Number formatting and plural-category preparation are
application runtime work, not part of this build phase.

## Features

| Feature selection | Available API and dependencies |
| --- | --- |
| Defaults | Bevy 0.20 runtime and `manifest` support; runtime loading defaults to Auto |
| `codegen` on the normal dependency | `translations!`, generated resources and manifest helpers; enables `manifest`, not generation |
| `build` alone, defaults disabled | Explicit generation in build.rs; generator build dependencies, no Bevy |
| `manifest` with a backend | Manifest constructors and the generator's runtime manifest API; no generation |
| One backend alone, defaults disabled | Byte sources and handwritten providers; no generator package, TOML or bridge |
| `watch` | Bevy asset-source file watching; does not watch custom byte loaders |
| `diagnostics` with a backend | Optional `progress` summaries and leaf diagnostics; disabled by default, no additional dependencies or systems |

`codegen` and `manifest` are independent of how you obtain runtime bytes.
Generated providers currently include manifest helpers, so `codegen` enables
those types even when the application chooses `from_bytes` or `from_loader`.
Disabling only `codegen` leaves the default `manifest` feature enabled. Use
`default-features = false` and select a backend for the minimal handwritten path.

## Manifest configuration

`[package.metadata.localization]` contains only `catalog`, the TOML file path
relative to the consuming package's `Cargo.toml`. Parent components (`..`) may
locate shared translation sources. The manifest selects `source-language`,
`default-language` and the optional `translations-directory` (default `"."`).
`languages-directory` is the legacy alias; do not specify both directory names.
Unknown fields in Cargo metadata and the TOML manifest are ignored; recognized
fields are validated. Languages are discovered from directories; keep their module/message contracts
in sync. Typed annotations belong to the source language.

The generated `CATALOG_CONFIG` contains the TOML contract, and `CATALOG_PATH`
is its package-relative build location. The generator does not select a Bevy
asset root or rewrite this path into an asset address. `texts::manifest()` keeps
that origin verbatim. The application must configure a source that resolves it,
or create a `LocalizationManifest` with a different runtime origin. For example,
with the standard Bevy `assets` root, use `LocalizationManifest::parse(
texts::CATALOG_CONFIG, "localizations/localization.toml")`.

No generated FTL payload is embedded unless the application
explicitly invokes `embed_manifest!`, including a declaration such as
`texts::embed_manifest! { const HUD = presentation::Hud; }`.
The Bevy facade re-exports the same `LocalizationManifest` type used by the
generator; declared constants require no additional import, adapter or conversion.

## Work on local checkouts

Local paths are for editing and testing the libraries together.
Replace the quickstart's runtime/build entries and add a generator patch as
shown below. Keep its other dependencies, metadata, `build.rs` and source files.
Set both paths to your checkouts:

```toml
[dependencies]
bevy_fluent_typed = { path = "/absolute/path/to/bevy_fluent_typed", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { path = "/absolute/path/to/bevy_fluent_typed", default-features = false, features = ["build"] }

[patch.crates-io]
fluent_typed_codegen = { path = "/absolute/path/to/fluent_typed_codegen" }
```

The runtime checkout contains the Bevy build adapter. Put the patch in the
workspace root when the consumer belongs to a workspace. This is development
wiring; normal applications use the registry dependencies in Setup.

The runtime requires generator 0.2.2 for its current generated contract. For local
changes in both libraries, select the matching generator checkout; cloning the
two repositories does not apply this override automatically.

For repeated example runs, add this to `.cargo/config.toml` at the runtime
repository root, preserving any existing settings:

```toml
[patch.crates-io]
fluent_typed_codegen = { path = "/absolute/path/to/fluent_typed_codegen" }
```

This local configuration applies the override to the commands in the example
READMEs. Keep it out of commits. If your enclosing workspace already supplies
the matching patch, no additional configuration is needed.

For a single command, provide the same patch on the command line instead:

```sh
cargo run --manifest-path examples/codegen/Cargo.toml --config 'patch.crates-io.fluent_typed_codegen.path="/absolute/path/to/fluent_typed_codegen"'
```
