# bevy_fluent_typed

Typed Fluent messages for Bevy. Generate typed accessors in `build.rs`, then read
translations through Bevy's asset system or explicitly embed them.

**0.2.0 (Unreleased):** this guide describes `feat/runtime-module-loading`. The
published runtime/bridge 0.1.3 and generator 0.1.4 have a different loading API.
The Git revisions below are the tested implementation, with package versions
from before the 0.2.0 bump. Version 0.2.0 is not published.

## Contents

- [Quickstart: create a project](#development-setup)
- [Add translations](#add-the-translations)
- [Run the application](#run-a-complete-application)
- [Embed translations](#explicit-embedding)
- [Supported Bevy versions](#supported-engines)
- [Full, Lazy and hybrid recipes](GUIDE.md)
- [Troubleshooting](#troubleshooting)
- [Known limits](#known-limits)
- [Examples and further reading](#where-to-go-next)

## Development setup

Start with `cargo new localized-app`, then work inside `localized-app`.
Use Rust 1.95 or newer. Replace `Cargo.toml` with:

```toml
[package]
name = "localized-app"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy = { version = "0.19", default-features = false, features = ["std", "async_executor", "multi_threaded", "bevy_asset", "bevy_text", "bevy_ui", "bevy_sprite"] }
bevy_fluent_typed = { git = "https://github.com/SDA-31/bevy_fluent_typed", rev = "46a2bb962c39c74447691cf76d26bd95c92ac963", features = ["codegen"] } # feat/runtime-module-loading

[build-dependencies]
bevy_fluent_typed = { git = "https://github.com/SDA-31/bevy_fluent_typed", rev = "46a2bb962c39c74447691cf76d26bd95c92ac963", default-features = false, features = ["build"] } # feat/runtime-module-loading

[patch.crates-io]
fluent_typed_codegen = { git = "https://github.com/SDA-31/fluent_typed_codegen", rev = "0e5ed447cc08553818a240fc6cfe736aa67fb4b3" }

[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"
```

The patch selects the matching unreleased generator. Put it in the **workspace
root** if this application belongs to a workspace. Edition 2024 uses Cargo
resolver 3; an explicit workspace must use resolver 2 or 3. Keep the same crate
name in both dependency sections. Enable the Bevy backend only on the normal
dependency, and `build` only on the build-dependency.

Create `build.rs` beside `Cargo.toml`:

```rust,ignore
fn main() -> std::process::ExitCode {
    bevy_fluent_typed::build()
}
```

This call reads the schema and generates Rust in Cargo's output directory.
The `translations!` macro below only includes that output. It does not discover
files, run generation or embed translation text.

## Add the translations

Create this layout. Each FTL file is a module that can load independently:

```text
assets/localizations/localization.toml
assets/localizations/translations/en/presentation/hud.ftl
assets/localizations/translations/en/screens/pause.ftl
assets/localizations/translations/es/presentation/hud.ftl
assets/localizations/translations/es/screens/pause.ftl
```

`assets/localizations/localization.toml`:

```toml
source-language = "en"
default-language = "en"
translations-directory = "translations"
```

`assets/localizations/translations/en/presentation/hud.ftl`:

```ftl
title = Ready
```

`assets/localizations/translations/es/presentation/hud.ftl`:

```ftl
title = Listo
```

`assets/localizations/translations/en/screens/pause.ftl`:

```ftl
title = Paused
```

`assets/localizations/translations/es/screens/pause.ftl`:

```ftl
title = En pausa
```

All languages must have matching modules, messages, arguments and references.
Typed argument annotations belong to the source language. This layout produces
`texts::presentation::Hud`, `texts::screens::Pause`, their parent types
`texts::Presentation` and `texts::Screens`, and the root `texts::Translations`.

## Run a complete application

Replace `src/main.rs` with this headless application. It loads from files, prints
`Ready` and exits. No window, font or GPU setup is needed:

```rust,ignore
use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    prelude::*,
};
use bevy_fluent_typed::{
    Localization, LocalizationManifest, LocalizationPlugin, ModuleStatus,
};
use std::{path::Path, time::Duration};

bevy_fluent_typed::translations!(mod texts);

fn main() -> AppExit {
    let manifest = LocalizationManifest::parse(
        texts::CATALOG_CONFIG,
        texts::CATALOG_ASSET_PATH,
    )
    .expect("valid localization manifest");
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join(texts::ASSET_ROOT);

    App::new()
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))),
            AssetPlugin {
                file_path: assets.to_string_lossy().into_owned(),
                ..default()
            },
            LocalizationPlugin::<texts::Translations>::new(manifest),
        ))
        .add_systems(Update, show_title)
        .run()
}

fn show_title(
    hud: Option<Res<texts::presentation::Hud>>,
    localization: Res<Localization<texts::Translations>>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(hud) = hud {
        println!("{}", hud.msg_title());
        exit.write(AppExit::Success);
    } else if let ModuleStatus::Failed(error) = localization.status::<texts::presentation::Hud>() {
        eprintln!("{error}");
        exit.write(AppExit::error());
    }
}
```

Run `cargo run`. The first build downloads dependencies. Commit the resulting
`Cargo.lock` for an application; later runs can use `cargo run --locked`.

The plugin defaults to **Full**: it requests all modules for the selected
language. `Option<Res<Hud>>` handles the time before that module is ready.
A normal application keeps running and can also read the independent pause
resource when it arrives. See the loading guide for **Lazy**, which requests
nothing until your code calls `load::<Scope>()`.

The `manifest` value is a `LocalizationManifest`: a parsed source contract,
not loaded translations. `parse` reads the supplied TOML string and performs no
file I/O. Its second argument is the manifest's **Bevy asset address**, used to
resolve FTL paths. With this configuration, the HUD's English file is
`assets/localizations/translations/en/presentation/hud.ftl`.

Build metadata chooses schema inputs; it does not force the runtime asset root.
The example anchors runtime assets to the package directory for reproducible
runs. A deployed application should supply its own asset root. Runtime TOML
is not watched: this example intentionally uses the contract prepared at build
time. To choose other runtime storage, pass a different parsed contract/origin.
`LocalizationManifest::from_file(path)` reads TOML through the native filesystem;
its filesystem origin is not automatically a Bevy-relative asset address.

## Explicit embedding

To use the same example without shipping FTL files, replace the manifest
initializer with:

```rust,ignore
let manifest = texts::embed_manifest!();
```

Keep `AssetPlugin` installed. The generated macro includes FTL bytes only where
it is invoked; generation alone does not embed them. This form includes every
known language's raw FTL. The selected language is parsed during plugin updates,
so readiness checks still apply. Static source bytes stay in the executable for
its lifetime; no compressor or decompressor is involved.

For a Lazy application that only needs the HUD, use
`texts::embed_manifest!(module = "presentation/hud.ftl")` and request only that
leaf. This embeds that file across known languages; it does not supply the pause
module. Group selection is not supported. The macro is crate-local even when
`texts` is public; a library can expose a function returning its chosen manifest.

## Supported engines

| Normal-dependency feature | Bevy release family |
| --- | --- |
| `bevy-0-19` (default) | 0.19.0 and compatible patches |
| `bevy-0-18` | 0.18.0 and compatible patches |
| `bevy-0-17` | 0.17.0 and compatible patches |
| `bevy-0-16` | 0.16.1 and compatible patches |

Select exactly one backend. For an older backend, disable defaults on the normal
runtime dependency, enable that backend plus `codegen`, and select the same
version family for `bevy`. Leave the build-dependency unchanged. Do not use
runtime/workspace `--all-features`. The example above uses Bevy 0.19 message APIs;
Bevy 0.16 uses `EventWriter`/`send` instead of `MessageWriter`/`write`.

Bevy 0.19 enforces immutable generated resources in ECS. Older backends expose
the same read-only catalog API without that ECS guarantee. `watch` is an
optional normal-dependency feature for filesystem change notifications.

## Troubleshooting

| What happened | What to check |
| --- | --- |
| `translations!` cannot find generated output | Add the shown build-dependency and return `bevy_fluent_typed::build()` from `build.rs`. |
| Types or methods are missing | Add the corresponding FTL module/message in every language and rebuild. File edits at runtime cannot change the compiled schema. |
| The resource is absent | File loading is asynchronous. Use `Option<Res<_>>` or a `resource_exists` run condition. In Lazy, explicitly request the scope first. |
| A bound label stays empty | Check `localization.status::<YourLeaf>()`. Its module must be requested and pass validation. A root binding waits for the whole tree. |
| Files are not found | The manifest origin is relative to Bevy's asset root. Do not prefix it with `assets/` when `AssetPlugin` already points there. |
| Editing the TOML has no runtime effect | The plugin receives a parsed contract and does not reload TOML. Rebuild this quickstart or construct a new contract during application setup. |
| Cargo reports incompatible engine APIs | Select one matching Bevy backend, on the normal dependency only. Do not enable all features. |
| A published release rejects these calls | Use the pinned Git dependencies above. Registry 0.1.3 has the previous API. |

## Known limits

A leaf is one complete FTL file in memory. Split large catalogs into modules and
use [Lazy loading](GUIDE.md#fully-lazy-complete-mainrs) to release unused ones.
Explicit embedding keeps static source bytes for the executable's lifetime.

Files publish independently, without automatic fallback or a multi-file
transaction. See [reload guarantees](GUIDE.md#scheduling-and-reload-guarantees)
for overlapping watcher/custom-reader requests. Number formatting, storage
transports, fonts and shaping remain application responsibilities.

## Where to go next

- [Loading guide](GUIDE.md): Full, Lazy, hybrid, typed resources, deferred text,
  language switching, hot reload and migration.
- [Custom asset sources](docs/asset-sources.md): archives, network-backed readers
  and named Bevy sources.
- [Number formatting](docs/formatting.md): application-owned ICU4X formatters.
- [Build API](docs/build.md): explicit generation and dependency feature isolation.

Runnable headless examples live in [examples/codegen](examples/codegen),
[examples/no_codegen](examples/no_codegen), [examples/minimal](examples/minimal),
[examples/icu](examples/icu) and [examples/asset_source](examples/asset_source).
Their README commands start from a checkout of this repository. Their local path
dependencies test that checkout; use the Git setup above for your application.
The [compatibility runner](tools/compatibility/README.md) is for maintainers.

Message accessors and Fluent resolution use
[fluent-typed](https://github.com/human-solutions/fluent-typed).
[MIT](LICENSE) covers this library, its bridge and examples, not your application
or translations. Package versions have not changed on this development branch.
