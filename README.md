# bevy_fluent_typed

[![crates.io](https://img.shields.io/crates/v/bevy_fluent_typed)](https://crates.io/crates/bevy_fluent_typed)
[![docs.rs](https://img.shields.io/docsrs/bevy_fluent_typed)](https://docs.rs/bevy_fluent_typed/latest/bevy_fluent_typed/)
[![CI](https://img.shields.io/github/actions/workflow/status/SDA-31/bevy_fluent_typed/ci.yml?branch=main&label=CI&logo=github)](https://github.com/SDA-31/bevy_fluent_typed/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/crates/msrv/bevy_fluent_typed)](https://crates.io/crates/bevy_fluent_typed)
[![License](https://img.shields.io/crates/l/bevy_fluent_typed)](LICENSE)

Typed Fluent integration for Bevy 0.17, 0.18 and 0.19. The runtime owns active languages,
asset loading, transactional reload, shared module resources and Text/Text2d bindings.
Applications own message keys, fonts, controls and generator settings; languages
are supplied by their catalog provider rather than a fixed runtime list.

Optional typed API generation is powered by
[fluent_typed_codegen](https://github.com/SDA-31/fluent_typed_codegen), which discovers
modular Fluent files and generates their Rust translation tree. The companion
`bevy_fluent_codegen_bridge` connects that tree to this runtime's resources and plugin.

Choose exactly one backend: `bevy-0-19` (default), `bevy-0-18` or `bevy-0-17`.
Each accepts patches in its own minor, starting at .0, not arbitrary future versions.
The application lockfile chooses the concrete patch release. The declared minimum
Rust version is 1.95 for both the runtime and its companion bridge.

[API documentation](https://docs.rs/bevy_fluent_typed/latest/bevy_fluent_typed/) ·
[Guide](GUIDE.md) · [Runnable example](examples/minimal/README.md) ·
[Optional bridge](codegen_bridge/README.md)

The crate's Rustdoc landing page contains a self-contained asset-to-resource
quick start. Its `texts::presentation::Hud` / `Res` sample is included from the
[runnable typed-resource example](docs/typed_resources.rs), not
maintained as a separate code copy. Read it on
[docs.rs](https://docs.rs/bevy_fluent_typed/0.1.0/bevy_fluent_typed/)
or build it locally with `cargo doc`.

## Optional generation

Without `codegen` this package is a standalone runtime for a `FluentCatalog`
provider. Enable `codegen` for the generated-provider integration and
`translations!`; enable `watch` for Bevy's file watcher. Only `bevy-0-19` is on by default.

For an older engine, set `default-features = false` and enable its backend, for
example `features = ["bevy-0-17", "codegen", "watch"]`. Your application's own Bevy
dependency must use the same minor. No engine flag belongs on the build bridge:
generated resources use the runtime's selected backend. Selecting no backend or
several backends is a compile error; do not use `--all-features` for this package.

Put dependencies and package metadata in your application's Cargo.toml.
No Git dependency, sibling checkout or registry patch is required.

```toml
[dependencies]
bevy_fluent_typed = { version = "0.1.0", features = ["codegen", "watch"] }

[build-dependencies]
bevy_fluent_codegen_bridge = { version = "0.1.0", features = ["build"] }

[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"
```

The bridge is a separate Cargo package in this repository. It adapts
`fluent_typed_codegen` output to this runtime. The runtime enables its lightweight `runtime`
feature; the build-dependency enables `build`. Use Cargo resolver 2 or 3 to keep
host build features separate from normal dependencies. The generator is not
compiled into the application runtime.

In `build.rs`:

```rust
fn main() -> std::process::ExitCode {
    bevy_fluent_codegen_bridge::build()
}
```

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
they are ECS-immutable resources: `ResMut` is rejected. On 0.17/0.18, Bevy does not
offer that resource-level guarantee; use `Res` and do not replace individual
modules. Only the central localization resource changes the active language.

Publication runs before Startup, in PreUpdate's `LocalizationSystems::Publish`,
and in PostUpdate before `LocalizationSystems::Refresh` text consumers.
Unchanged reloads and inactive-language edits do not replace active resources.

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

- Exact Bevy 0.17.0, 0.18.0 and 0.19.0 on Linux with stable Rust.
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
