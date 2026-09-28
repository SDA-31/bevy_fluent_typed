# bevy_fluent_typed

Typed Fluent messages for Bevy. Generate typed accessors in `build.rs`, then read
translations through Bevy's asset system or explicitly embed them.

[Migrate from 0.1.3](docs/migration-0.2.md) · [Resource waiting in 0.2.1](docs/migration-0.2.1.md) · [Changelog](CHANGELOG.md)

## Contents

- [Quickstart: create a project](#setup)
- [Add translations](#add-the-translations)
- [Run the application](#run-a-complete-application)
- [Embed translations](#explicit-embedding)
- [Supported Bevy versions](#supported-engines)
- [Full, Lazy and hybrid recipes](GUIDE.md)
- [Troubleshooting](#troubleshooting)
- [Known limits](#known-limits)
- [Examples and further reading](#where-to-go-next)

## Setup

Start with `cargo new localized-app`, then work inside `localized-app`.
Use Rust 1.95 or newer. Replace `Cargo.toml` with:

```toml
[package]
name = "localized-app"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy = { version = "0.19", default-features = false, features = ["std", "async_executor", "multi_threaded", "bevy_asset", "bevy_text", "bevy_ui", "bevy_sprite"] }
bevy_fluent_typed = { version = "0.2.1", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.2.1", default-features = false, features = ["build"] }

[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"
```

Runtime 0.2.1 uses the compatible 0.2.0 bridge and generator from crates.io; no patch
is needed. Edition 2024 uses Cargo resolver 3; an explicit workspace must use
resolver 2 or 3. Keep the same crate name in both dependency sections. Enable
the Bevy backend only on the normal dependency, and `build` only on the build-dependency.

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
    Localization, LocalizationAppExt, LocalizationManifest, LocalizationPlugin, ModuleStatus,
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
        .add_localized_startup_systems(show_title)
        .add_systems(Update, report_failure)
        .run()
}

fn show_title(
    hud: Res<texts::presentation::Hud>,
    mut exit: MessageWriter<AppExit>,
) {
    println!("{}", hud.msg_title());
    exit.write(AppExit::Success);
}

fn report_failure(
    localization: Res<Localization<texts::Translations>>,
    mut exit: MessageWriter<AppExit>,
) {
    if let ModuleStatus::Failed(error) = localization.status::<texts::presentation::Hud>() {
        eprintln!("{error}");
        exit.write(AppExit::error());
    }
}
```

Run `cargo run`. The first build downloads dependencies. Commit the resulting
`Cargo.lock` for an application; later runs can use `cargo run --locked`.

The plugin defaults to **Full**: it requests all modules for the selected
language. `add_localized_startup_systems` waits for the required `Res<Hud>`
and runs the function once after loading. The frame keeps running while it waits.
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

A Lazy application can select just its HUD by type:

```rust,ignore
let manifest = texts::embed_manifest!(module = texts::presentation::Hud);
```

Request the same leaf through
`localization.load::<texts::presentation::Hud>()`.
The manifest contains its bytes across known languages, without the pause module.
Select `texts::Presentation` for that group's descendants, or
`texts::Translations` for the whole tree. `use` imports, including `as` aliases,
work; arbitrary `type` aliases and generic parameters do not select recipes.

Unselected payloads are absent even in debug builds without optimization, LTO
or linker dead-code removal. The macro is crate-local even when `texts` is public;
a library can expose a function returning its chosen manifest.

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
| A typed `embed_manifest!(module = ...)` selector is rejected | Use version 0.2.1 from [Setup](#setup) in both dependency sections and remove old overrides. Pass a generated path or `use` alias, not a string or `type` alias. |
| The resource is absent | Use `add_localized_systems` with native `Res<_>` to wait. In Lazy, request the scope first and keep that request active while the screen needs it. |
| A bound label stays empty | Check `localization.status::<YourLeaf>()`. Its module must be requested and pass validation. A root binding waits for the whole tree. |
| Files are not found | The manifest origin is relative to Bevy's asset root. Do not prefix it with `assets/` when `AssetPlugin` already points there. |
| Editing the TOML has no runtime effect | The plugin receives a parsed contract and does not reload TOML. Rebuild this quickstart or construct a new contract during application setup. |
| Cargo reports incompatible engine APIs | Select one matching Bevy backend, on the normal dependency only. Do not enable all features. |
| A release rejects these calls | Loading changed in [0.2.0](docs/migration-0.2.md); resource-waiting helpers require [0.2.1](docs/migration-0.2.1.md). Keep both dependency versions aligned. |

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
- [Migration](docs/migration-0.2.md): update an existing 0.1.3 application.
- [Changelog](CHANGELOG.md): notable changes grouped by release.

Runnable headless examples live in [examples/codegen](examples/codegen),
[examples/no_codegen](examples/no_codegen), [examples/minimal](examples/minimal),
[examples/icu](examples/icu) and [examples/asset_source](examples/asset_source).
Their README commands start from a checkout of this repository. Their local path
dependencies test that checkout; use the registry setup above for your application.
The [compatibility runner](tools/compatibility/README.md) is for maintainers.
CI ignores branch pushes and PRs changing only `CHANGELOG.md`; tag pushes and
manual runs still execute checks.

Message accessors and Fluent resolution use
[fluent-typed](https://github.com/human-solutions/fluent-typed).
[MIT](LICENSE) covers this library, its bridge and examples, not your application
or translations.
