# bevy_fluent_typed

[![crates.io](https://img.shields.io/crates/v/bevy_fluent_typed)](https://crates.io/crates/bevy_fluent_typed)
[![docs.rs](https://img.shields.io/docsrs/bevy_fluent_typed)](https://docs.rs/bevy_fluent_typed/latest/bevy_fluent_typed/)
[![CI](https://img.shields.io/github/actions/workflow/status/SDA-31/bevy_fluent_typed/ci.yml?branch=main&label=CI&logo=github)](https://github.com/SDA-31/bevy_fluent_typed/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/crates/msrv/bevy_fluent_typed)](https://crates.io/crates/bevy_fluent_typed)
[![License](https://img.shields.io/crates/l/bevy_fluent_typed)](LICENSE)

Typed [Fluent](https://projectfluent.org/) localization for Bevy. Turn modular
translation files into Rust accessors, request a catalog as a Bevy `Res`, and
keep existing `Text`/`Text2d` entities up to date when the language changes.

- Typed message access and arguments through [fluent-typed](https://github.com/human-solutions/fluent-typed).
- Shared catalog resources for the whole translation tree, a folder or one file.
- Atomic reloads that preserve the last working translation if an edit is invalid.
- Optional API generation with [fluent_typed_codegen](https://github.com/SDA-31/fluent_typed_codegen), or your own `FluentCatalog` provider.

Your application chooses message keys, languages, fonts and controls. The runtime
handles asset loading, language selection and text bindings.

[API documentation](https://docs.rs/bevy_fluent_typed/latest/bevy_fluent_typed/) ·
[Guide](GUIDE.md) · [Runnable examples](#minimal-examples) ·
[Companion bridge](codegen_bridge/README.md)

## Contents

- [Getting started](#getting-started)
- [Compatibility and features](#compatibility-and-features)
- [Minimal examples](#minimal-examples)
- [Runtime integration](#runtime-integration)
- [Decimal numbers, plurals and RTL](#decimal-numbers-plurals-and-rtl)
- [Reload and provider boundaries](#reload-and-provider-boundaries)
- [Verification](#verification) and [continuous integration](#continuous-integration)
- [License](#license)

<a id="optional-generation"></a>

## Getting started

This setup adds generated catalogs to an existing Bevy application. For a complete
consumer you can run immediately, start with [examples/codegen](examples/codegen).

### 1. Add dependencies and source paths

The setup below uses the public build facade introduced in **0.1.1**. Both
dependency entries use this same crate; 0.1.0 consumers should upgrade to use it.
Add these entries to your application's `Cargo.toml`:

```toml
[dependencies]
bevy_fluent_typed = { version = "0.1.2", features = ["codegen", "watch"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.1.2", default-features = false, features = ["build"] }

[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"
```

Use Cargo resolver 2 or 3 to keep build and runtime features separate. The build
dependency disables defaults so it compiles no Bevy; normal runtime use compiles
no generator. The companion bridge is an internal adapter, so application
manifests and `build.rs` use only the public `bevy_fluent_typed` facade.

Select engine features directly on the normal dependency. Forwarding them through
application features to the shared dependency name also enables them in the host
graph. Renaming the dependency is supported; use the same alias in both sections.

### 2. Generate during the build

In `build.rs`:

```rust
fn main() -> std::process::ExitCode {
    bevy_fluent_typed::build()
}
```

This explicit call generates the API under Cargo's `OUT_DIR` and registers
catalog files and directories for build-script reruns. `translations!` only
includes the prepared output; macro expansion never runs the generator or writes
files. A normal dependency cannot supply dependencies to your `build.rs`.

After an asset edit, upstream's watched staging timestamps can cause one
additional build-script run; subsequent unchanged checks are verified to stay fresh.

<a id="source-assets-and-generated-tree"></a>

### 3. Create the translation assets

```text
assets/localizations/
  localization.toml
  translations/
    en/presentation/hud.ftl
    es/presentation/hud.ftl
```

In `assets/localizations/localization.toml`:

```toml
translations-directory = "translations"
source-language = "en"
default-language = "en"
```

In `en/presentation/hud.ftl`:

```ftl
title = Flight HUD
# $name (String) - Pilot name supplied by the application.
detail = Pilot { $name }
```

In `es/presentation/hud.ftl`:

```ftl
title = Panel de vuelo
detail = Piloto { $name }
```

Locale directories are discovered automatically. Give every language the same
module/message contract, with type annotations in the source language. Paths and
the source/startup languages are configurable. `asset-root` is relative to your
package, `catalog` to the asset root, and `translations-directory` to the TOML file.

Omit `translations-directory` when locale folders sit beside the TOML; the default
is `"."`. `languages-directory` remains a legacy alias, but do not set both names.
Renaming the key without changing its value, or spelling out the default `"."`,
preserves the reload contract. Changing the resolved directory requires regeneration.

### 4. Register the plugin and use typed messages

Declare the tree without naming an output file:

```rust
bevy_fluent_typed::translations!(pub mod texts);

use texts::{Locale, Translations};
```

The bridge owns output filenames and hygienic dependency aliases; no handwritten
adapter or generated source file belongs in the source tree. The macro does not
replace `build.rs` or install the runtime plugin.

For the `assets/` layout above, Bevy's `DefaultPlugins` already uses the correct
asset directory; no extra path configuration is needed. Add
`LocalizationPlugin::<Translations>::new(texts::CATALOG_ASSET_PATH)` after
`DefaultPlugins`.

Set `AssetPlugin.file_path` only if your application loads its assets from a
different directory, such as `Resources/`. The `asset-root` setting in Cargo.toml
tells the generator where to find translations **during the build**; it does not
configure where Bevy loads files **while the application runs**. The catalog path
is relative to Bevy's asset directory: `localizations/localization.toml`, without
the `assets/` prefix.

The plugin makes generated modules available as resources before `Startup`.
For the files above, a system can request the HUD directly:

```rust
use bevy::prelude::*;
use bevy_fluent_typed::LocalizedText;

fn show_hud(mut commands: Commands, hud: Res<texts::presentation::Hud>) {
    println!("{}", hud.msg_title());

    commands.spawn((
        Text::default(),
        LocalizedText::<Translations>::new(|catalog| {
            catalog.presentation().hud().msg_detail("Ada")
        }),
    ));
}
```

Register the system in `Startup`. `LocalizedText` also works with `Text2d` and
updates the existing component when the catalog or binding changes. Call
`Localization<Translations>::set_locale(Locale::Es)` to switch to Spanish.

The [typed-resource example](examples/minimal/src/bin/typed_resources.rs) contains
the complete, tested application setup. For a windowed application, supply your
normal UI hierarchy, camera and fonts. Choose a deployment asset root explicitly
when packaging the application.

## Compatibility and features

Choose exactly one backend: `bevy-0-19` (default), `bevy-0-18`, `bevy-0-17` or `bevy-0-16`.
Each accepts patches in its own minor, not arbitrary future versions. The minimum
is 0.16.1 for the oldest backend and .0 for the others. The 0.16 compatibility
check uses that engine patch with its required `bevy_color` 0.16.2; an all-0.16.0
exact dependency set cannot be freshly resolved because `bevy_color` 0.16.0 is yanked.
The application lockfile chooses the concrete patch release. The declared minimum
Rust version is 1.95 for both the runtime and its companion bridge.
This is **stable Rust**, not nightly. Bevy 0.19 itself requires
[Rust 1.95](https://github.com/bevyengine/bevy/blob/v0.19.0/Cargo.toml), so the
default backend adds no compiler-version requirement beyond the engine's.
Selecting an older backend does not currently lower this crate's declared MSRV;
older compiler support would need its own dependency and CI checks.

The `bevy-0-16` backend and `CatalogUpdateReader` alias are available starting with
0.1.2; version 0.1.1 supports Bevy 0.17–0.19. Bevy release candidates are not covered by
the stable compatibility promise.

Compiler and engine support are separate decisions: a Rust minimum increase does
not by itself remove a Bevy backend. When support for a compiler or backend is
dropped, the last compatible release will be documented. Older releases remain
available, without a promise of indefinite maintenance.

| Feature | Purpose |
| --- | --- |
| `bevy-0-19` (default), `bevy-0-18`, `bevy-0-17`, `bevy-0-16` | Select exactly one Bevy backend |
| `codegen` | Generated-provider integration and the `translations!` macro |
| `watch` | Bevy's filesystem watcher for live edits |
| `build` | Explicit generation from the consumer's build script |
| `runtime` | Runtime APIs; enabled automatically by each backend |

For an older engine, set `default-features = false` and enable its backend, for
example `features = ["bevy-0-17", "codegen", "watch"]`. Your application's own Bevy
dependency must use the same minor. The build-only facade needs no engine flag:
generated resources use the runtime's selected backend. Runtime use requires exactly
one backend; `default-features = false, features = ["build"]` needs none.
**Do not use `--all-features` for this package:** it selects incompatible backends.

Without `codegen`, use a handwritten `FluentCatalog` provider as shown in the
[no-codegen example](examples/no_codegen). Without either `codegen` or `build`,
the runtime compiles neither the companion bridge nor the generator.

## Minimal examples

| Example | What it demonstrates |
| --- | --- |
| [With codegen](examples/codegen) | One module per language, generated `texts::ui::Greeting`, EN/ES/RU switching |
| [Without codegen](examples/no_codegen) | A handwritten `FluentCatalog`, automatic `Text` updates, no build script or build dependencies |
| [Typed resources](examples/minimal/src/bin/typed_resources.rs) | Chained catalog access, `Res<texts::presentation::Hud>`, localized text and language switching |
| [Integration suite](examples/minimal) | Typed arguments, resource scopes, filesystem watching and contract regression tests |
| [ICU formatters](examples/icu) | Shared Decimal/percentage services and EN/ES/RU/AR text updates |

From a clone of this repository:

```sh
cargo run --manifest-path examples/codegen/Cargo.toml
cargo run --manifest-path examples/no_codegen/Cargo.toml
cargo run --manifest-path examples/icu/Cargo.toml
```

Repository examples use local paths to test their checkout. Use the registry
dependencies in [Getting started](#getting-started) for your application.

## Runtime integration

`Message<Translations>` stores typed formatting closures and owned arguments for
deferred rendering. `LocalizedText<Translations>` binds a message to an existing
`Text`/`Text2d`; the application owns fonts, input and error presentation.

For `presentation/hud.ftl`, borrow `translations.presentation().hud()` as
`&texts::presentation::Hud`, or request `Res<texts::presentation::Hud>`.
Root, groups and leaves share one read-only snapshot through `Arc`. On Bevy 0.19,
they are ECS-immutable resources: `ResMut` is rejected. On 0.16–0.18, Bevy does not
offer that resource-level guarantee; use `Res` and do not replace individual
modules. Only the central localization resource changes the active language.

Publication runs before Startup, in PreUpdate's `LocalizationSystems::Publish`,
and in PostUpdate before `LocalizationSystems::Refresh` text consumers.
Unchanged reloads and inactive-language edits do not replace active resources.

## Decimal numbers, plurals and RTL

For localized numbers, percentages, currencies and dates, it's recommended to use
a dedicated formatting library, such as [ICU4X](https://docs.rs/icu/) or
[ICU](https://unicode-org.github.io/icu/userguide/format_parse/). The choice belongs
to your application: the runtime integrates translations with Bevy, but does not
implement number formatting or depend on ICU.

Already formatted text can be passed to a `(String)` FTL argument. If plural
selection should follow the displayed decimal precision, one approach is to pass
the same prepared Decimal to ICU4X DecimalFormatter and PluralRules, then supply
the category keyword through a separate String selector. Native Fluent numeric
selectors remain available as an alternative.

The standalone [ICU resource example](examples/icu) creates Decimal and percentage
formatters once per locale, injects them through `Res`, and captures a shared
`Arc` in deferred bindings. It covers English, Spanish, Russian and Arabic.
No runtime feature, new trait implementation or public API change is needed.
The percentage component is explicitly pinned experimental ICU4X code, confined
to the example; its input is percent units, so a ratio is scaled before formatting.

Fluent matches String selectors to literal `[one]`, `[few]`, etc., with the
starred branch as fallback. Native numeric selectors and exact `[0]`/`[1]`
matches remain supported separately. See the
[plural guide](GUIDE.md#decimal-and-plural-arguments) for the FTL contract and
the [compiled deferred-text test](examples/minimal/src/tests/plurals.rs) for
automatic updates when the active language changes.

Capture the raw Decimal and reusable per-locale formatters in deferred messages,
not a formatted string prepared for an old language. Select the formatter from
the current catalog's locale each time the closure renders. Replace the binding
when the numeric value or formatting policy changes. A change to an arbitrary
Bevy resource is not an automatic invalidation signal for captured `Arc`s.

Arabic digits and grammatical rules do not provide complete RTL UI support.
Preserve Fluent's bidi isolation; glyph shaping, visual bidi ordering, fonts,
alignment and mirrored layout belong to the application's rendering stack.

## Reload and provider boundaries

The runtime reads an opaque definition asset and asks the provider for a
`CatalogDescriptor`. It does not parse TOML. The bridge validates generator
configuration semantically; comments, whitespace and field order are accepted.
The runtime validates relative asset addresses and preserves named asset sources.

Each language is published atomically. Invalid sources preserve last-known-good
text; other valid languages can update independently. `CatalogUpdate` reports
outcomes and `ReloadCatalogs` requests another load. Initially missing modules
need explicit reload after creation. Read-but-invalid modules remain watched.

A provider's `parse` must check the entire module inventory and typed contract.
Generated providers check exact keys/references, then upstream typed/structured
message contracts. Configuration, schema, language or module changes require
regeneration and restart; compatible prose edits hot reload. Embedded catalogs
remain usable when files are absent.

## Verification

From a standalone clone, use explicit manifests for the runtime, bridge and example.
The generator dependency resolves from crates.io:

```sh
git clone https://github.com/SDA-31/bevy_fluent_typed.git
cd bevy_fluent_typed
cargo test --manifest-path Cargo.toml --features codegen,watch
cargo test --manifest-path codegen_bridge/Cargo.toml --all-features
cargo test --manifest-path examples/minimal/Cargo.toml
cargo run --manifest-path examples/minimal/Cargo.toml
cargo run --manifest-path examples/minimal/Cargo.toml --bin typed_resources
cargo test --manifest-path examples/icu/Cargo.toml
cargo run --manifest-path examples/icu/Cargo.toml
```

Standalone lockfiles and target directories are local build artifacts. Use
`--locked --offline` after resolving dependencies once. Library manifests do not
assume a generator sibling path. For local generator development, add the caller-owned override
`--config 'patch.crates-io.fluent_typed_codegen.path="/absolute/path/to/checkout"'`.

If a consuming workspace lists these packages as members, use its generator
override and lockfile instead. For a workspace including the runtime, bridge,
generator and minimal example:

```sh
cargo run --locked --offline -p localization-example
cargo run --locked --offline -p localization-example -- --watch
cargo test --locked --offline --workspace
cargo clippy --locked --offline --workspace --all-targets -- -D warnings
cargo doc --locked --offline -p bevy_fluent_typed -p bevy_fluent_codegen_bridge -p fluent_typed_codegen --features bevy_fluent_typed/codegen,bevy_fluent_typed/watch,bevy_fluent_codegen_bridge/build --no-deps
```

See the guide for typed arguments, scheduling and custom providers. Verify feature
isolation in separate consumer graphs. Never unify the mutually exclusive engine
backends with `--all-features`.

Maintainers can run the [Rust compatibility tool](tools/compatibility/README.md)
to pin and test exact engine releases in disposable library-only workspaces.

## Continuous integration

[CI workflow](.github/workflows/ci.yml) runs on pushes (including tags), pull
requests and manual dispatch, without an enclosing application checkout:

- Exact Bevy 0.16.1, 0.17.0, 0.18.0 and 0.19.0 on Linux with stable Rust.
- Bevy 0.19.0 on Windows/macOS with stable Rust and on Linux with Rust 1.95.0.
- Real multi-file watcher reloads, headless consumers, text-system ordering,
  runtime dependency isolation and the resource-mutability compile probes.
- Formatting of every package, Clippy, bridge/tool tests, runtime-only bridge
  isolation, Rustdoc and library archive inventories on Linux.

The generator is checked out separately at `GENERATOR_REV`, a full
commit SHA in the workflow. Push that commit to its Git repository **before**
pushing this workflow; update the pin deliberately when adopting a new generator.
The caller-owned path override exists only in CI commands and disposable fixtures,
not in library manifests. No repository credentials or game source are needed.

Standalone lockfiles are resolved before `--locked` checks. Compatibility jobs
use the Rust maintainer tool, preserve their console logs for seven days, and
limit concurrent matrix jobs to three. These are headless tests, not rendering tests.

Actions are SHA-pinned with read-only repository permission and no retained
checkout credentials. There is **no automatic publication**, registry token or
release creation. Pushing version tags to GitHub triggers tests only.
Archive inventory checks are not a successful `cargo publish --dry-run`.
Manual releases verify registry-backed packaging and publish dependencies in order:
`fluent_typed_codegen`, then `bevy_fluent_codegen_bridge`, then `bevy_fluent_typed`.

## License

[MIT](LICENSE). The companion bridge and minimal example are also MIT-licensed,
with their own package metadata and license files. This license covers this
repository, not a consuming application or its assets. Third-party dependencies retain
their respective licenses.
