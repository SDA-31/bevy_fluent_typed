# Migrate from bevy_fluent_typed 0.2.1 to 0.2.2

Plugin constructors, explicit Full/Manual requests, typed resources and readiness
helpers keep their signatures. This guide covers generation paths and selective
embedding; the current 0.3 default is Lazy. See [automatic loading](migration-0.3.md#adopt-automatic-module-loading)
for the default change and explicit Full compatibility.

## Update both facade dependencies

```toml
[dependencies]
bevy_fluent_typed = { version = "0.3.0", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.3.0", default-features = false, features = ["build"] }
```

These dependency snippets target runtime 0.3.0. Follow the
[0.3 migration](migration-0.3.md) to select Bevy 0.20 or retain an older backend.
The facade requires generator 0.2.2; no separate bridge package is needed.
Keep `bevy_fluent_typed::build()` in
`build.rs` and the `translations!` declaration. Remove local overrides when
switching to registry dependencies.

## Separate the build path from the asset origin

Replace the former `asset-root` plus `catalog` with one Cargo-package path:

```toml
[package.metadata.localization]
catalog = "assets/localizations/localization.toml"
```

The generator resolves this relative to `Cargo.toml`, independently of Bevy.
The TOML's `translations-directory` is still relative to that TOML. Unknown
fields are ignored; known values and directory alias conflicts are checked.
A leftover `asset-root` has no effect and does not prefix `catalog`.

`texts::manifest()` remains a no-argument helper. Its origin is now the verbatim
package-relative `CATALOG_PATH`, rather than `CATALOG_ASSET_PATH`. Generated
`ASSET_ROOT` is removed. Choose the mapping matching your deployment:

- Keep `texts::manifest()` and configure `AssetPlugin.file_path` to the package
  or installation base containing `assets/`; the quickstart uses `"."`.
- Keep Bevy's standard `assets` root and supply its asset-relative origin:

```rust,ignore
let manifest = bevy_fluent_typed::LocalizationManifest::parse(
    texts::CATALOG_CONFIG,
    "localizations/localization.toml",
)?;
app.add_plugins(LocalizationPlugin::<texts::Translations>::new(manifest));
```

Named sources follow the same rule: archive keys or virtual source addresses
are supplied by the application, not inferred from the build path. Native
`LocalizationManifest::from_file` reads filesystem paths; it does not configure
AssetServer. See [custom sources](asset-sources.md).

## Use named embedded constants

Replace selective macro arguments such as
`texts::embed_manifest!(module = texts::presentation::Hud)` with declarations:

```rust,ignore
texts::embed_manifest! {
    pub const HUD = presentation::Hud;
}
```

Use the manifest in plugin setup and retain catalog aliases for resource systems:

```rust,ignore
use texts::presentation::Hud as Interface;
app.add_plugins(LocalizationPlugin::<texts::Translations>::new(HUD));
```

`HUD` has type `LocalizationManifest`. Relative selectors belong to the generated
schema; ordinary imports and aliases remain usable for `Res<Interface>` and
catalog construction. The no-argument `texts::embed_manifest!()` form remains.
Leaf-name helper macros and selector expression arguments are removed.

Embedding includes only selected source bytes, including in debug builds.
Lazy consumers or explicit Full/Manual govern parsing and module lifetime; selected static bytes remain
for the executable's lifetime. Byte inputs and native resource waiting retain
their APIs; follow [0.3 language switching](migration-0.3.md#let-language-changes-finish-automatically)
for the active-until-ready `set_locale` contract. Test file/ZIP origins and locale
switches after migration. See the [changelog](../CHANGELOG.md).

## Typed configuration and build errors

Configuration parsers now return `ConfigError`, and explicit generation functions
return `BuildError` instead of `String`. The standard `build() -> ExitCode` call
is unchanged. Update custom `Result<(), String>` signatures to the typed error,
or convert to a string deliberately at the application boundary.

`ManifestError::Config { source }` replaces string configuration/path diagnostics;
`ManifestError::RequiresLoader { origin }` identifies virtual manifest origins.
The error enums are non-exhaustive: include a wildcard arm in application matches.
Underlying parser, I/O and upstream causes are retained through `Error::source()`.
