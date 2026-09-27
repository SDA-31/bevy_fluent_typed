## Explicit build-script API

For a complete Cargo.toml, files and application, follow the
[quickstart](https://github.com/SDA-31/bevy_fluent_typed/blob/main/README.md#setup).
This section explains the optional custom build entrypoints.

The `build` feature exposes `build()`, `from_cargo()`, `generate()` and
`Settings` through the same crate used by the application. The companion bridge
is an implementation detail; consumers do not need to name it in Cargo.toml.
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

Cargo feature forwarding such as `bevy_fluent_typed/bevy-0-17` can enable a feature
on both dependency kinds when they share a name. Select the backend directly in
the **normal dependency's** `features` array, not through application feature
forwarding. Cargo also requires the same dependency name in both sections; if
renaming the crate, use that alias in both, as in the integration example.

The public facade is available since **0.1.1**. Version 0.1.0 required a separate
bridge build-dependency. Number formatting and plural-category preparation are
application runtime work, not part of this build phase.

## Manifest configuration

`[package.metadata.localization]` selects the build-time `asset-root` and the
manifest's asset-relative `catalog` path. The manifest selects `source-language`,
`default-language` and the optional `translations-directory` (default `"."`).
`languages-directory` is the legacy alias; do not specify both directory names.
Languages are discovered from directories; keep their module/message contracts
in sync. Typed annotations belong to the source language.

The generated `CATALOG_CONFIG` contains the TOML contract, and `CATALOG_ASSET_PATH`
is its build-time asset address. Reusing them at runtime is convenient, but
optional: the application can pass another compatible source contract and choose
another asset root. No generated FTL payload is embedded unless the application
explicitly invokes `embed_manifest!()`.

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

The runtime checkout supplies its own companion bridge. Put the patch in the
workspace root when the consumer belongs to a workspace. This is development
wiring; normal released consumers use the registry quickstart.

For repository examples using a local generator, provide this same
patch on the command line (replace the path with your generator checkout):

```sh
cargo run --manifest-path examples/codegen/Cargo.toml --config 'patch.crates-io.fluent_typed_codegen.path="/absolute/path/to/fluent_typed_codegen"'
```
