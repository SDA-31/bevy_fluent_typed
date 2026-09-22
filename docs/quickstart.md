Typed Fluent localization for Bevy: language switching, hot-reloaded assets,
typed module resources and automatic updates to existing `Text`/`Text2d` entities.

This Git-only branch additionally offers `bevy-0-20` for exactly Bevy
0.20.0-rc.1 with Rust 1.96+. It is not a crates.io release; the stable quick start
below still uses Bevy 0.19. Preview consumers must use the Git dependency and
disable defaults. See the repository's `docs/compat-bevy-0.20.md` for setup.

Message access and Fluent resolution use
[fluent-typed](https://docs.rs/fluent-typed/0.9.0/fluent_typed/).

Optional typed API generation is powered by
[fluent_typed_codegen](https://docs.rs/fluent_typed_codegen/0.1.4/fluent_typed_codegen/),
which discovers modular Fluent files and generates their Rust translation tree.
The companion
[bevy_fluent_codegen_bridge](https://docs.rs/bevy_fluent_codegen_bridge/0.1.3/bevy_fluent_codegen_bridge/)
connects that tree to this runtime's resources and plugin.

Turn `presentation/hud.ftl` into `texts::presentation::Hud`. Borrow it through
chained accessors or request it directly as a Bevy `Res` — no string keys or
handwritten catalog adapter.

Bevy **0.16, 0.17, 0.18 and 0.19** are supported with explicit backends, not arbitrary
future Bevy releases. The declared minimum Rust version is **1.95 stable**;
nightly is not required. This matches
[Bevy 0.19's own minimum](https://github.com/bevyengine/bevy/blob/v0.19.0/Cargo.toml).
Older backends currently retain the same crate-level MSRV; they do not promise
compatibility with older Rust compilers. Fonts, glyph coverage, layout and window
setup belong to your app.

Support for 0.16 and the `CatalogUpdateReader` alias starts with 0.1.2;
version 0.1.1 supports 0.17–0.19. Release candidates are outside the stable
compatibility promise.

## Features

| Feature | What it adds |
| --- | --- |
| `bevy-0-19` (default) | Bevy 0.19.0 and compatible patches; ECS-immutable generated resources |
| `bevy-0-20` (Git-only preview) | Exactly Bevy 0.20.0-rc.1; Rust 1.96+; ECS-immutable generated resources |
| `bevy-0-18` | Bevy 0.18.0 and compatible patches |
| `bevy-0-17` | Bevy 0.17.0 and compatible patches |
| `bevy-0-16` | Bevy 0.16.1 and compatible patches; `watch` also enables its required multithreaded executor |
| `codegen` | The `translations!` macro and companion generated-provider integration |
| `watch` | Bevy's filesystem watcher for live text edits |
| `build` | Explicit build-script generation; disable defaults for an engine-free host build |
| `runtime` | Runtime APIs, enabled automatically by each Bevy backend |

The generator runs in the consuming application's build script, not every frame.
Without `codegen` or `build`, this runtime does not compile the bridge or generator.

Select **exactly one** engine backend. For 0.16, 0.17 or 0.18, set
`default-features = false` and enable the matching feature alongside any optional
`codegen`/`watch` features. Your direct Bevy dependency must use the same minor.
Build-only use (`default-features = false, features = ["build"]`) needs no backend.
`--all-features` is intentionally
invalid for this runtime because it selects incompatible backends together.

All backends expose read-only catalog snapshots. Only Bevy 0.19 can enforce
resource immutability in ECS: `ResMut<texts::presentation::Hud>` is rejected there.
On older backends, use `Res` by convention; do not replace individual modules.
`ResMut<Localization<Translations>>` remains supported on every backend for
language selection. Changing a `LocalizedText` binding does not need a mutable catalog.

## Quick start: from assets to typed resources

### 1. Dependencies and source paths

This setup uses the public build facade introduced in **0.1.1**. Upgrade both
dependency entries together from 0.1.0. The runnable repository examples use
path dependencies only to test their checkout.

In your application's Cargo.toml:

```toml
[dependencies]
bevy = { version = "0.19.0", default-features = false, features = ["std", "async_executor", "multi_threaded", "bevy_asset", "bevy_text", "bevy_ui", "bevy_sprite"] }
bevy_fluent_typed = { version = "0.1.3", features = ["codegen", "watch"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.1.3", default-features = false, features = ["build"] }

[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"
```

Use resolver 2 or 3 to keep build and runtime features separate. Cargo.lock pins
the resolved versions. The bridge is an implementation detail; application code
and both dependency entries use `bevy_fluent_typed`. No registry patch is required.

In `build.rs`:

```rust,ignore
fn main() -> std::process::ExitCode {
    bevy_fluent_typed::build()
}
```

Generation runs during `cargo check` and rust-analyzer's build-script indexing.
Output stays in Cargo's `OUT_DIR` under target/, never beside translation assets.
The explicit build.rs call performs all generation and registers source tracking.
`translations!` only includes the prepared output; macro expansion does not invoke
the generator or write files. A normal dependency feature cannot declare build
dependencies for the consuming package.

Small, complete consumers are available separately:
[with codegen](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/codegen)
and [without codegen](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/no_codegen).
The latter has no build.rs and implements a one-message `FluentCatalog` by hand.

### 2. Create the assets

```text
assets/
  localizations/
    localization.toml
    translations/
      en/presentation/hud.ftl
      es/presentation/hud.ftl
```

`assets/localizations/localization.toml`:

```toml
translations-directory = "translations"
source-language = "en"
default-language = "en"
```

`translations-directory` is optional and relative to this TOML. Omit it when
locale folders (`en/`, `es/`, etc.) sit beside the file; its default is `"."`.
The legacy `languages-directory` alias is accepted, but do not set both names.

`en/presentation/hud.ftl`:

```ftl
title = Flight HUD
# $name (String) - Pilot name supplied by the application.
detail = Pilot { $name }
```

`es/presentation/hud.ftl`:

```ftl
title = Panel de vuelo
detail = Piloto { $name }
```

Languages and modules are discovered automatically. All languages must provide
the same module/message contract; type annotations belong to the source language.
The language directory is relative to the TOML, whose path is relative to the
asset root. The asset root is relative to the consuming package at build time.

### 3. Use the generated names

| Source | Generated Rust API |
| --- | --- |
| Whole translation tree | `texts::Translations` |
| `presentation/` folder | `texts::Presentation` and namespace `texts::presentation` |
| `presentation/hud.ftl` file | `texts::presentation::Hud` |
| `title` in that file | `hud.msg_title()` |
| `detail` with `$name (String)` | `hud.msg_detail("Ada")` |

A file is a **leaf type**, not another public module: there is no
`texts::presentation::hud`. Identically named messages in different files remain
independent. For example, another `ui/menu.ftl` becomes `texts::ui::Menu`.

### 4. Register the plugin and request a module

Configure Bevy's asset root and add
`LocalizationPlugin::<texts::Translations>::new(texts::CATALOG_ASSET_PATH)` after
Bevy's `AssetPlugin`. Register your UI system in `Startup`; its
`Res<texts::presentation::Hud>` parameter receives the generated module resource.

The complete [headless example](https://github.com/SDA-31/bevy_fluent_typed/blob/main/examples/minimal/src/bin/typed_resources.rs)
shows asset configuration, the
`Translations → Presentation → Hud` chain, `Res<texts::presentation::Hud>`, a
localized Bevy text entity and switching to Spanish. With a window, use your
normal Bevy plugins and UI hierarchy; localization bindings work the same way.

The executable source and its test live together in
`examples/minimal/src/bin/typed_resources.rs`, not in this library's documentation.
The example requires the consumer's build script and FTL files above; the bundled
consumer also includes Russian and more test modules.
The source-tree asset path is convenient for development; choose a deployment
asset root explicitly when packaging your application.
