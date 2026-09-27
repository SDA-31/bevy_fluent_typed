# bevy_fluent_typed

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

## Full, Lazy and hybrid loading

`Localization<C, Full>` and `LocalizationPlugin<C, Full>` use Full by default.
Full requests every module of the selected locale. It exposes no load/unload API.
Lazy starts without demand:

```rust,ignore
use bevy_fluent_typed::{Lazy, Localization, LocalizationPlugin};
app.add_plugins(LocalizationPlugin::<texts::Translations, Lazy>::new(manifest));
// Equivalent initializer:
// LocalizationPlugin::<texts::Translations>::new_lazy(manifest)

fn enter(mut localization: ResMut<Localization<texts::Translations, Lazy>>) {
    localization.load::<texts::Presentation>();
    localization.load::<texts::presentation::Hud>();
}

fn leave(mut localization: ResMut<Localization<texts::Translations, Lazy>>) {
    localization.unload::<texts::Presentation>(); // Hud's independent request remains.
}
```

Requests are idempotent per scope type. Root requests need every leaf; group
requests need descendants. Overlap is the union of outstanding requests. Hybrid
loading means retaining a base scope request and loading other scopes as needed.
A repeated failed `load` retries; accessors themselves never initiate loading.
`Full` and `Lazy` implement the sealed `LoadingMode` trait.

```compile_fail,E0599
use bevy_fluent_typed::{FluentCatalog, Full, Localization};
fn request<C: FluentCatalog>(state: &mut Localization<C, Full>) {
    state.load::<C>();
}
```

```compile_fail,E0599
use bevy_fluent_typed::{FluentCatalog, Full, Localization};
fn release<C: FluentCatalog>(state: &mut Localization<C, Full>) {
    state.unload::<C>();
}
```

```compile_fail
use bevy_fluent_typed::{FluentCatalog, FluentScope, Lazy, Localization};
fn wrong_root<C: FluentCatalog, S: FluentScope>(state: &mut Localization<C, Lazy>) {
    state.load::<S>(); // S::Catalog must be C.
}
```

```compile_fail
use bevy_fluent_typed::LoadingMode;
struct Other;
impl LoadingMode for Other { const FULL: bool = false; }
```

## Resources, navigation and bindings

Complete root/group/leaf scopes are published as shared immutable resources.
A leaf can exist while its parent/root is incomplete. `localization.catalog()`
returns `Option<&C>`. Schema navigation remains available while unloaded:

```rust,ignore
let hud = localization.modules().presentation().hud()?;
// `hud` borrows the controller's store, not a temporary view.
let title = hud.msg_title();
let message = Message::new(|hud: &texts::presentation::Hud| hud.msg_title());
let binding = LocalizedText::<texts::presentation::Hud>::new(|hud| hud.msg_title());
```

Group navigation returns another view. Leaf access returns `Result<&Leaf,
ModuleError>` with locale, logical path and latest loading status. `status::<S>()`
reports Unloaded/Loading/Ready/Failed. A failed reload can still have an available
previous valid same-language value; availability and last-attempt status differ.
`Message<S>`/`LocalizedText<S>` support leaf, group and root with no mode generic.
Bindings update existing Text/Text2d entities, clearing text when their scope is
unavailable and refreshing when inserted. They never request modules. Keep
editable drafts separate and replace bindings when captured arguments change.

## Locale changes, failures and ownership

`set_locale` keeps requests but clears previous-language state and handles.
No automatic fallback or language cache exists. Startup follows the manifest's
known default language unless a controller was inserted before the plugin.
A mismatching source language or unknown default is rejected explicitly.
Only one plugin mode may own a given root provider in an App.

Successful checked leaves publish independently. A same-language invalid reload
retains that leaf's last good value; valid siblings can still update. Switching
languages cannot reuse that old value as fallback. Every successful reload
produces a fresh snapshot, including identical text. Idle frames and unchanged
siblings keep resource identity and Bevy change ticks.

`CatalogUpdate::Loaded { locale, path }` and `Rejected { locale, path, error }`
identify individual module attempts. Observe via `CatalogUpdateReader` after
`LocalizationSystems::Publish`. Send `ReloadCatalogs` to retry demanded FTL,
including initially missing files. It does not reread TOML: the supplied manifest
is an immutable contract. Bevy 0.16 uses EventWriter/send_event; newer backends
use MessageWriter/write_message. A read-but-invalid file remains watched and may
recover automatically when `watch` and its source support watching.

Localization-owned reloads are serialized per module, including requests queued
while the initial read is pending; repeated pending retries coalesce. Loader
revisions also prevent an earlier loader from replacing a later accepted result
when their `read_to_end` futures complete in reverse order.

Automatic watcher reloads and direct application `AssetServer::reload` calls are
outside that queue. Bevy opens an `AssetReader` before invoking the loader and
exposes no request generation at that boundary. If an older reader-opening
future completes after a newer one, chronology cannot be recovered by this
integration. `AssetLoadFailedEvent` likewise carries no request generation,
including failures inside the loader's `read_to_end`. Strict ordering therefore
requires disabling automatic watching and routing reloads through `ReloadCatalogs`;
custom sources must also
provide coherent data. This crate does not wrap the application's reader or
perform a second I/O pass to infer freshness.

Publication occurs in PreUpdate's Publish and again before PostUpdate's Refresh.
Even embedded parsing starts during an update, not plugin initialization. Update
language changes reach resources in PostUpdate; order consumers after Refresh
for synchronized text. The controller itself changes immediately, so earlier
same-frame direct resources can still represent the preceding publication.

Unloading the final request drops runtime strong handles, leaves and ancestor
snapshots at synchronization; Bevy may finish retiring assets on later updates.
Caller-owned clones can intentionally retain parsed resources. Embedded static
source bytes cannot be freed. File-backed Lazy mode reads and retains only demand;
choose reasonably sized FTL leaves when a language contains gigabytes of text.
No hidden source String copy is retained just to compare reloads.

## Migration from registry 0.1.3

Pass a `LocalizationManifest` instead of a path string to the plugin. Choose an
explicit source, and replace implicit embedded startup assumptions with resource
availability checks. `Locale::load`, `Translations::embedded`, provider
`descriptor` and whole-language `parse` hooks are removed. Core accessors and
Arc-backed scope types remain; direct bytes use `Leaf::new` or root
`Translations::from_modules`, both checked by default. Safe `_unchecked` core
constructors skip schema validation only, still checking UTF-8 and Fluent syntax.
Separate validation methods permit checking data before loading it.

Handwritten providers implement `FluentScope` and `FluentCatalog`, declaring
checked `Module::new::<Leaf>` parsers and `ScopeRegistration::new::<Scope>`
assembly hooks. Assembly shares ready children and must not reread or reparse.
See the no_codegen example for the smallest provider. Numeric formatting,
fonts, shaping, storage transports and error presentation remain application policy.

## Examples and verification

The five headless examples live under `examples/`: `codegen` (explicit embedding),
`no_codegen` (handwritten provider and file source), `minimal` (generated resource
and watcher suite), `icu` (application-owned formatters), `asset_source` (named
memory reader). Run using each Cargo manifest and the generator patch above.
Local example path dependencies test this checkout; they are not installation instructions.

See [GUIDE.md](GUIDE.md), [numeric formatting](docs/formatting.md),
[asset sources](docs/asset-sources.md), and the
[exact-backend maintainer runner](tools/compatibility/README.md).
The runner verifies isolated host/runtime graphs and exact 0.16.1/0.17.0/0.18.0/0.19.0
families; 0.16.1 requires bevy_color 0.16.2. Do not run --all-features.

MIT covers this library, nested bridge and examples. It does not license your
application or translations. No package version has changed in this branch.
