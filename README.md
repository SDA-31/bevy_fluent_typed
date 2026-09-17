# bevy_fluent_typed

Typed Fluent integration for Bevy 0.17, 0.18 and 0.19. The runtime owns active languages,
asset loading, transactional reload, shared module resources and Text/Text2d bindings.
Applications own message keys, fonts, controls and generator settings; languages
are supplied by their catalog provider rather than a fixed runtime list.

Choose exactly one backend: `bevy-0-19` (default), `bevy-0-18` or `bevy-0-17`.
Each accepts patches in its own minor, starting at .0, not arbitrary future versions.
The application lockfile chooses the concrete patch release. The declared minimum
Rust version is 1.95 for both the runtime and its companion bridge.

[Guide](GUIDE.md) · [Runnable example](examples/minimal/README.md) ·
[Optional bridge](codegen_bridge/README.md)

The crate's Rustdoc landing page contains a self-contained asset-to-resource
quick start. Its `texts::presentation::Hud` / `Res` sample is included from the
[runnable typed-resource example](docs/typed_resources.rs), not
maintained as a separate code copy. Build it locally with `cargo doc`; docs.rs
will show the same content once packages are published on crates.io.

## Optional generation

Without `codegen` this package is a standalone runtime for a `FluentCatalog`
provider. Enable `codegen` for the generated-provider integration and
`translations!`; enable `watch` for Bevy's file watcher. Only `bevy-0-19` is on by default.

For an older engine, set `default-features = false` and enable its backend, for
example `features = ["bevy-0-17", "codegen", "watch"]`. Your application's own Bevy
dependency must use the same minor. No engine flag belongs on the build bridge:
generated resources use the runtime's selected backend. Selecting no backend or
several backends is a compile error; do not use `--all-features` for this package.

Install from [GitHub](https://github.com/SDA-31/bevy_fluent_typed); the packages are
not yet published on crates.io. Put dependencies and package metadata in your
application's Cargo.toml, and `[patch.crates-io]` in the workspace root (the same
file for a standalone application). A virtual workspace has no package sections.
Cargo.lock pins the selected Git revisions; for explicit pins,
replace `branch` with `rev` (use the same Bevy-repository revision for both packages).

```toml
[dependencies]
bevy_fluent_typed = { git = "https://github.com/SDA-31/bevy_fluent_typed.git", branch = "main", features = ["codegen", "watch"] }

[build-dependencies]
bevy_fluent_codegen_bridge = { git = "https://github.com/SDA-31/bevy_fluent_typed.git", branch = "main", features = ["build"] }

# Required for generation until the generator is published on crates.io.
# Patches belong to the consuming workspace root, not this library.
[patch.crates-io]
fluent_typed_codegen = { git = "https://github.com/SDA-31/fluent_typed_codegen.git", branch = "main" }

[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"
```

The bridge is a separate Cargo package in this repository. It adapts
`fluent_typed_codegen` output to this runtime. The runtime enables its lightweight `runtime`
feature; the build-dependency enables `build`. Use Cargo resolver 2 or 3 to keep
host build features separate from normal dependencies. The generator is not
compiled into the application runtime.

These packages are available from Git, not crates.io. Keep the root generator
override until crates.io publication: Cargo can resolve optional dependencies
when creating a lockfile even if their features are disabled. Runtime-only builds
do not compile the generator; dependency resolution and feature activation are
different. The Git patch does not require a sibling checkout. After publication,
the registry version can replace this override.

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
Fetch the unpublished generator from Git using a caller-owned override:

```sh
git clone https://github.com/SDA-31/bevy_fluent_typed.git
cd bevy_fluent_typed
cargo test --manifest-path Cargo.toml --features codegen,watch --config 'patch.crates-io.fluent_typed_codegen.git="https://github.com/SDA-31/fluent_typed_codegen.git"' --config 'patch.crates-io.fluent_typed_codegen.branch="main"'
cargo test --manifest-path codegen_bridge/Cargo.toml --all-features --config 'patch.crates-io.fluent_typed_codegen.git="https://github.com/SDA-31/fluent_typed_codegen.git"' --config 'patch.crates-io.fluent_typed_codegen.branch="main"'
cargo test --manifest-path examples/minimal/Cargo.toml --config 'patch.crates-io.fluent_typed_codegen.git="https://github.com/SDA-31/fluent_typed_codegen.git"' --config 'patch.crates-io.fluent_typed_codegen.branch="main"'
cargo run --manifest-path examples/minimal/Cargo.toml --config 'patch.crates-io.fluent_typed_codegen.git="https://github.com/SDA-31/fluent_typed_codegen.git"' --config 'patch.crates-io.fluent_typed_codegen.branch="main"'
cargo run --manifest-path examples/minimal/Cargo.toml --bin typed_resources --config 'patch.crates-io.fluent_typed_codegen.git="https://github.com/SDA-31/fluent_typed_codegen.git"' --config 'patch.crates-io.fluent_typed_codegen.branch="main"'
```

Standalone lockfiles and target directories are local build artifacts. Use
`--locked --offline` after resolving dependencies once. Library manifests do not
assume a generator sibling path; the override is caller-owned. For local generator
development, replace the Git/branch override with
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

## License

[MIT](LICENSE). The companion bridge and minimal example are also MIT-licensed,
with their own package metadata and license files. This license covers this
repository, not a consuming application or its assets. Third-party dependencies retain
their respective licenses.
