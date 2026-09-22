# Custom asset source with codegen

Load a generated `texts::ui::Hud` resource through a named Bevy asset source.
The headless application prints `Ready` after the source has loaded, then exits.
It uses Bevy's built-in `MemoryAssetReader`, not a ZIP reader. This example targets
the default Bevy 0.19 backend and adds no archive dependency.

## Run

From this repository:

```sh
cargo run --manifest-path examples/asset_source/Cargo.toml
cargo test --manifest-path examples/asset_source/Cargo.toml
```

## Start here

1. [Cargo.toml](Cargo.toml) enables `codegen` on the normal dependency and `build`
   on the build-dependency, with defaults disabled there.
2. [build.rs](build.rs) calls `bevy_fluent_typed::build()` explicitly.
3. [main.rs](src/main.rs) includes the generated `texts` module, registers the
   source, adds `LocalizationPlugin` and reads `Res<texts::ui::Hud>`.
4. [source.rs](src/source.rs) supplies the virtual files for this self-contained
   example. A real archive source supplies its own bytes instead.

The application registers `"translations"` **before** `AssetPlugin`, then passes
`"translations://localizations/localization.toml"` to the localization plugin.
This is Bevy's ordinary `source://path` syntax; `translations` is a name chosen by
the application, not a built-in protocol or archive format. The TOML and its FTL
modules are all read through that source.

`show_title` waits for `CatalogUpdate::Loaded` so it demonstrates a real load
through the reader, not just the embedded fallback available at startup.
It prints the typed HUD message and exits. The headless schedule runner replaces
a window's event loop; no manual polling or test bookkeeping is needed in main.

## Using another storage format

Replace the reader factory in `main.rs` with your asset plugin's reader.
The localization plugin and generated resources do not change. Build-time FTL
files still define the API and embedded fallback; code generation does not read
the runtime archive.

After installing an updated pack, send `ReloadCatalogs::<texts::Translations>`.
For pack layout, consistent revisions and watcher responsibilities, see
[custom asset sources](../../docs/asset-sources.md).

The memory source starts from generated `MODULES` solely to make this example
self-contained. It is not an independently distributed translation pack.
Older Bevy backends have the same localization contract, but Bevy 0.16/0.17 use
`AssetSourceBuilder::default().with_reader(...)`; Bevy 0.16 uses events instead of
messages.

## Verification

[tests/](tests) contains explicit reloads, language switching, rejection/recovery
scenarios, `Text`/`Text2d` consistency checks, bounded test polling and update
fixtures. These are not additional resources or steps required by an application.
Run them with `cargo test`; `cargo run` performs no test mutations.

Local path dependencies test this checkout. Public installation instructions are
in the main [README](../../README.md#getting-started).

[MIT](LICENSE).
