# bevy_fluent_typed

[![crates.io](https://img.shields.io/crates/v/bevy_fluent_typed)](https://crates.io/crates/bevy_fluent_typed)
[![docs.rs](https://img.shields.io/docsrs/bevy_fluent_typed)](https://docs.rs/bevy_fluent_typed/latest/bevy_fluent_typed/)
[![CI](https://img.shields.io/github/actions/workflow/status/SDA-31/bevy_fluent_typed/ci.yml?branch=main&label=CI&logo=github)](https://github.com/SDA-31/bevy_fluent_typed/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/crates/msrv/bevy_fluent_typed)](https://crates.io/crates/bevy_fluent_typed)
[![License](https://img.shields.io/crates/l/bevy_fluent_typed)](LICENSE)

Typed Fluent integration for Bevy 0.16, 0.17, 0.18 and 0.19, built on
[fluent-typed](https://github.com/human-solutions/fluent-typed) for typed message
access and Fluent resolution. The runtime owns active languages,
asset loading, transactional reload, shared module resources and Text/Text2d bindings.
Applications own message keys, fonts, controls and generator settings; languages
are supplied by their catalog provider rather than a fixed runtime list.

Optional typed API generation is powered by
[fluent_typed_codegen](https://github.com/SDA-31/fluent_typed_codegen), which discovers
modular Fluent files and generates their Rust translation tree. The companion
`bevy_fluent_codegen_bridge` connects that tree to this runtime's resources and plugin.

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

The `bevy-0-16` backend and `CatalogUpdateReader` alias are unreleased additions;
published 0.1.1 supports Bevy 0.17–0.19. Bevy release candidates are not covered by
the stable compatibility promise.

[API documentation](https://docs.rs/bevy_fluent_typed/latest/bevy_fluent_typed/) ·
[Guide](GUIDE.md) · [Codegen example](examples/codegen) · [No-codegen example](examples/no_codegen) ·
[Optional bridge](codegen_bridge/README.md)

The crate's Rustdoc landing page contains a self-contained asset-to-resource
quick start. Its `texts::presentation::Hud` / `Res` sample is included from the
[runnable typed-resource example](docs/typed_resources.rs), not
maintained as a separate code copy. Read it on
[docs.rs](https://docs.rs/bevy_fluent_typed/latest/bevy_fluent_typed/)
or build it locally with `cargo doc`.

## Optional generation

Without `codegen` this package is a standalone runtime for a `FluentCatalog`
provider. Enable `codegen` for the generated-provider integration and
`translations!`; enable `watch` for Bevy's file watcher. Only `bevy-0-19` is on by default.

For an older engine, set `default-features = false` and enable its backend, for
example `features = ["bevy-0-17", "codegen", "watch"]`. Your application's own Bevy
dependency must use the same minor. The build-only facade needs no engine flag:
generated resources use the runtime's selected backend. Runtime use requires exactly
one backend; `default-features = false, features = ["build"]` needs none.
Do not use `--all-features` for this package.

The setup below uses the public build facade introduced in **0.1.1**. Both
dependency entries use this same crate; 0.1.0 consumers should upgrade to use it.
Repository examples deliberately use local paths to test their checkout.

```toml
[dependencies]
bevy_fluent_typed = { version = "0.1.1", features = ["codegen", "watch"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.1.1", default-features = false, features = ["build"] }

[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"
```

Both dependency entries refer to the same public crate. Its `build` feature
exposes explicit generation; `codegen` exposes the runtime integration. The bridge
is an internal adapter, not a required name in application manifests or build.rs.
Use Cargo resolver 2 or 3: build-only use compiles no Bevy, while normal use
compiles no generator. Disable defaults on the build-dependency.

In `build.rs`:

```rust
fn main() -> std::process::ExitCode {
    bevy_fluent_typed::build()
}
```

This is the **explicit generation phase**. `translations!` only includes its
output; expanding the macro never invokes a generator or writes files. A normal
dependency feature cannot supply dependencies to the consumer's build.rs, so this
build-dependency is intentional. Cargo tracks catalog files and directories through
the bridge's build-script rerun directives.

Select engine features directly on the normal dependency. Forwarding them through
application features to the shared dependency name also enables them in the host
graph. After an asset edit, upstream's watched staging timestamps can cause one
additional build-script run; subsequent unchanged checks are verified to stay fresh.

## Minimal examples

- [With codegen](examples/codegen): one FTL module per language, the explicit
  build call above, generated `texts::ui::Greeting` and EN/ES/RU switching.
- [Without codegen](examples/no_codegen): no build.rs or build-dependencies;
  a small handwritten `FluentCatalog` and automatic updates to a Bevy `Text`.
- [Integration suite](examples/minimal): typed arguments, resource scopes,
  filesystem watching and contract regression tests.
- [ICU formatters](examples/icu): application-owned Decimal and percentage
  services, shared Bevy resources and EN/ES/RU/AR text updates.

```sh
cargo run --manifest-path examples/codegen/Cargo.toml
cargo run --manifest-path examples/no_codegen/Cargo.toml
cargo run --manifest-path examples/icu/Cargo.toml
```

## Source assets and generated tree

Continue the generated setup with the source assets below.

In `assets/localizations/localization.toml`:

```toml
translations-directory = "translations"
source-language = "en"
default-language = "en"
```

Add matching modular FTL trees below
`assets/localizations/translations/{en,es,ru}/`. All locale directories are
discovered. Both paths and the source/startup languages are configurable.
The example starts in English and includes Spanish and Russian translations.
Omit `translations-directory` when language folders sit beside the TOML file;
the default is `"."`. `languages-directory` remains a legacy alias, but do not set
both names. Renaming the key without changing its value, or replacing an omitted
directory with explicit `"."`, preserves the reload contract. Changing the resolved
directory requires regeneration.

Declare the tree without naming an output file:

```rust
bevy_fluent_typed::translations!(pub mod texts);

use texts::{Locale, Translations};
```

The facade forwards to the bridge, which owns output filenames and hygienic
dependency aliases. Renaming the runtime dependency in Cargo is supported.
Generated files stay under Cargo OUT_DIR; no handwritten adapter or generated
source file belongs in the source tree. The macro does not replace build.rs or
install the runtime plugin.

## Runtime integration

Configure `AssetPlugin.file_path`, then add
`LocalizationPlugin::<Translations>::new(texts::CATALOG_ASSET_PATH)`.
Use `LocalizedText<Translations>` with existing `Text`/`Text2d` components,
and `Localization<Translations>::set_locale` to switch languages.
`Message<Translations>` stores typed formatting and owned arguments for deferred
rendering. Fonts, input and error presentation belong to the application.

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

Use dedicated [ICU4X](https://docs.rs/icu/) or
[ICU](https://unicode-org.github.io/icu/userguide/format_parse/) components for
localized numbers, percentages, currencies and dates. The runtime integrates
translations with Bevy; it does not implement number formatting or add a direct
ICU dependency.
Pass already formatted text to a `(String)` FTL argument. When grammar must
follow decimal precision, pass the same prepared value to ICU4X DecimalFormatter
and PluralRules, then supply the category keyword through a separate String
selector. Keep native numeric selectors available where they fit the application.

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
