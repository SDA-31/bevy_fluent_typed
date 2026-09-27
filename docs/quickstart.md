Typed Fluent localization for Bevy with explicit module loading and typed resources.

**Unreleased API on `feat/runtime-module-loading`.** Registry runtime/bridge 0.1.3
and generator 0.1.4 use the previous loading API. No release has been published.
Message accessors and resolution use [fluent-typed](https://github.com/human-solutions/fluent-typed).

## Development setup

This temporary Git setup selects the matching generator revision. After a future
release, ordinary registry consumers will not need this development patch.

```toml
[dependencies]
bevy = { version = "0.19", default-features = false, features = ["std", "async_executor", "multi_threaded", "bevy_asset", "bevy_text", "bevy_ui", "bevy_sprite"] }
bevy_fluent_typed = { git = "https://github.com/SDA-31/bevy_fluent_typed", branch = "feat/runtime-module-loading", features = ["codegen", "watch"] }

[build-dependencies]
bevy_fluent_typed = { git = "https://github.com/SDA-31/bevy_fluent_typed", branch = "feat/runtime-module-loading", default-features = false, features = ["build"] }

[patch.crates-io]
fluent_typed_codegen = { git = "https://github.com/SDA-31/fluent_typed_codegen", rev = "0e5ed447cc08553818a240fc6cfe736aa67fb4b3" }

[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"
```

Use Cargo resolver 2/3. Select a backend only on the normal dependency; keep the
same dependency alias in both sections. Do not forward backend features through a
consumer feature to the shared dependency name: that also enables the host graph.
The bridge remains an implementation detail. Runtime support uses the generator
package's small manifest module, without its `build` feature or generation dependencies.

Your explicit `build.rs`:

```rust,ignore
fn main() -> std::process::ExitCode {
    bevy_fluent_typed::build()
}
```

The build step discovers schemas and generates Rust into `OUT_DIR`.
`translations!` only includes prepared output; it never generates or writes files.

## Source contract and generated types

```text
assets/localizations/localization.toml
assets/localizations/translations/en/presentation/hud.ftl
assets/localizations/translations/es/presentation/hud.ftl
```

```toml
source-language = "en"
default-language = "en"
translations-directory = "translations"
```

The optional directory defaults to `"."`; `languages-directory` is its legacy
alias, and both names together are rejected. Build languages must share module,
message, argument and reference contracts. Typed annotations belong to the source
language. `presentation/hud.ftl` generates `texts::presentation::Hud`, its folder
becomes `texts::Presentation`, and the root is `texts::Translations`.

```rust,ignore
bevy_fluent_typed::translations!(pub mod texts);
use bevy_fluent_typed::{LocalizationManifest, LocalizationPlugin};

let manifest = LocalizationManifest::parse(
    texts::CATALOG_CONFIG,
    "localizations/localization.toml",
)?;
app.add_plugins(LocalizationPlugin::<texts::Translations>::new(manifest));
```

Register after Bevy's `AssetPlugin`. `parse` constructs an immutable source
contract without reading FTL. Its origin is an asset address chosen by the app;
runtime storage may differ from build storage. `from_config` accepts an already
prepared config. `from_file` reads only TOML using the native filesystem, so its
origin is not automatically relative to Bevy's asset root. Prefer `parse` with an
explicit asset origin for Bevy, or prepare `from_config` yourself.

For a small application that wants all selected-language modules in memory, use
Full mode above and ordinary `Res<texts::presentation::Hud>`. Resources appear
when their modules are ready. Use `Option<Res<Hud>>` or a `resource_exists::<Hud>`
run condition while loading; an unconditional missing `Res` follows Bevy's system
validation behavior. Do not require asynchronously loaded resources in Startup.

## Explicit embedding

```rust,ignore
app.add_plugins(LocalizationPlugin::<texts::Translations>::new(texts::embed_manifest!()));
```

This explicit invocation expands deferred `include_bytes!` recipes. Without an
invocation, generated Rust does not reference FTL payloads; no linker stripping
assumption is needed. The no-argument form includes all raw source bytes across
languages; selected-language resources are parsed during plugin updates. Static
embedded bytes themselves remain for the executable's lifetime.
`texts::embed_manifest!(module = "presentation/hud.ftl")` includes only that leaf
across languages and can serve a Lazy request for it. Group selection is not
provided. The generated embedding macro is crate-local even in `pub mod texts`;
a library may expose a function returning its chosen embedded manifest.
No compressor/decompressor or implicit embedded fallback is installed.

## Supported engines

| Feature | Supported release family |
| --- | --- |
| `bevy-0-19` (default) | 0.19.0 and compatible patches; generated ECS-immutable resources |
| `bevy-0-18` | 0.18.0 and compatible patches |
| `bevy-0-17` | 0.17.0 and compatible patches |
| `bevy-0-16` | 0.16.1 and compatible patches; watch enables the required executor |

Rust minimum: **1.95 stable**. Select exactly one engine backend; disable defaults
for an older backend. Never use runtime/workspace `--all-features`. Older engines
retain read-only APIs without the 0.19 ECS immutability guarantee. The `build`
feature with defaults disabled needs no backend. `codegen` adds the declaration
macro, `watch` adds filesystem watching, and backends enable `runtime`.
