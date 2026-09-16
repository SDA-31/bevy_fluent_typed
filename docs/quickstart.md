Typed Fluent localization for Bevy: language switching, hot-reloaded assets,
typed module resources and automatic updates to existing `Text`/`Text2d` entities.

Use the optional directory generator to turn `presentation/hud.ftl` into
`texts::presentation::Hud`. Borrow it through chained accessors or request it
directly as a Bevy `Res` — no string keys or handwritten catalog adapter.

Bevy **0.19.0 and compatible 0.19.x patches** are supported, not arbitrary future
Bevy releases. The declared minimum Rust version is **1.95**. Fonts, glyph coverage,
layout and window setup belong to your app.

## Features

| Feature | What it adds |
| --- | --- |
| none (default) | Runtime for your own [`FluentCatalog`] provider |
| `codegen` | The `translations!` macro and companion generated-provider integration |
| `watch` | Bevy's filesystem watcher for live text edits |

The generator runs in the consuming application's build script, not every frame.
Without `codegen`, this runtime does not compile the bridge or generator.

## Quick start: from assets to typed resources

### 1. Dependencies and source paths

Until crates.io publication, use the Git repositories. Application Cargo.toml:

```toml
[dependencies]
bevy = { version = "0.19.0", default-features = false, features = ["std", "async_executor", "multi_threaded", "bevy_asset", "bevy_text", "bevy_ui", "bevy_sprite"] }
bevy_fluent_typed = { git = "https://github.com/SDA-31/bevy_fluent_typed.git", branch = "main", features = ["codegen", "watch"] }

[build-dependencies]
bevy_fluent_codegen_bridge = { git = "https://github.com/SDA-31/bevy_fluent_typed.git", branch = "main", features = ["build"] }

[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"

# This section belongs to the workspace root, which may be a different file.
[patch.crates-io]
fluent_typed_codegen = { git = "https://github.com/SDA-31/fluent_typed_codegen.git", branch = "main" }
```

Use resolver 2 or 3 to keep build and runtime features separate. Cargo.lock pins
the resolved Git revisions; replace `branch` with `rev` for explicit pins. The
runtime and bridge must use the same revision of their shared repository.

The patch supplies the unpublished generator to Cargo. Feature selection and
dependency resolution are different: disabled optional code need not compile,
but Cargo may still resolve its package when creating a lockfile.

In `build.rs`:

```rust,ignore
fn main() -> std::process::ExitCode {
    bevy_fluent_codegen_bridge::build()
}
```

Generation runs during `cargo check` and rust-analyzer's build-script indexing.
Output stays in Cargo's `OUT_DIR` under target/, never beside translation assets.

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

The following headless example shows asset configuration, the
`Translations → Presentation → Hud` chain, `Res<texts::presentation::Hud>`, a
localized Bevy text entity and switching to Spanish. With a window, use your
normal Bevy plugins and UI hierarchy; localization bindings work the same way.

The snippet is included directly from the runnable `typed_resources` example,
so the documentation and example use the **same Rust source**. It requires the
consumer's build script and FTL files above; it cannot run as an isolated doctest
of this library. The bundled consumer also includes Russian and more test modules.
The source-tree asset path is convenient for development; choose a deployment
asset root explicitly when packaging your application.
