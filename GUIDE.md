# Guide: typed localization with Bevy

The [README](README.md#optional-generation) contains the current dependency setup.
This guide covers the directory-generator integration; custom providers can use
the runtime without its optional `codegen` feature.

## 1. Configure sources

The example uses this configurable layout:

```text
assets/localizations/
  localization.toml
  translations/
    en/ui/menu.ftl
    es/ui/menu.ftl
    ru/ui/menu.ftl
```

Cargo metadata defines `asset-root = "assets"` and
`catalog = "localizations/localization.toml"`. The first path is relative to the
consuming package; the second is relative to the asset root. There is no scan
for an arbitrary resource folder. The definition filename is configurable.

```toml
translations-directory = "translations"
source-language = "en"
default-language = "en"
```

`translations-directory` is relative to this TOML and optional. Omit it to use
language folders beside the file (default `"."`). The legacy `languages-directory`
alias remains accepted; specifying both names is an error even with equal values.
Every immediate subdirectory is a language, so keep unrelated folders outside.
Nested FTL paths define scopes. Source-language defines the API and type
annotations; default-language selects startup and may differ from it.

Unknown fields, missing required language fields, nonstring or blank values, absolute/escaping paths,
backslashes, asset source/label syntax and symlinked source trees are rejected.
Keep the same module paths, keys and references in every language.

## 2. Add a typed message

Source `en/ui/menu.ftl`:

```ftl
# $name (String) - Player name supplied by the application.
menu-greeting = Hello, { $name }!
```

Spanish `es/ui/menu.ftl`:

```ftl
menu-greeting = ¡Hola, { $name }!
```

Russian `ru/ui/menu.ftl`:

```ftl
menu-greeting = Привет, { $name }!
```

The generated `msg_menu_greeting(name)` accessor is checked by Rust. Method
argument order follows first occurrence in the **source-language pattern**, not
comment order. Translation word order can differ: Fluent substitutes by name.

Keys are independent between files, including argument types. References to
messages, terms and attributes stay within the same file; unresolved/cyclic
references fail. Cross-file imports are not implemented. Production number
formatting belongs to the application: a displayed localized number may be passed
as a String, with a separate numeric selector if grammar requires it.

## 3. Generate and index

Enable `bevy_fluent_typed/codegen` at runtime and use
`bevy_fluent_codegen_bridge` with `build` in build-dependencies. Return its result:

```rust
fn main() -> std::process::ExitCode {
    bevy_fluent_codegen_bridge::build()
}
```

This emits readable errors without panicking. `from_cargo()` returns Result for
custom handlers. Do not ignore generation errors: output is not transactional.

Declare the module:

```rust
bevy_fluent_typed::translations!(pub mod texts);

use texts::{Locale, Translations};
```

Visibility, attributes, nested placement and renamed Cargo dependencies work.
The facade passes its runtime path to the bridge; the bridge owns the output
include. No handwritten generated.rs or FluentCatalog implementation is needed.

Outputs stay in Cargo OUT_DIR under the selected target/profile:

- `translations.rs`: engine-neutral tree.
- `validation.rs`: private strict schema checks, emitted from the generator.
- `locale_modules.rs`: metadata and original sources.
- `modules/<FTL path without extension>/translations.{rs,ftl}`: upstream modules.
- `bevy_catalog.rs`: bridge-owned resource tree and provider implementation.
- `inputs/<inventory hash>/`: persistent staging inputs for Cargo reruns.

Do not include both trees into the same scope or duplicate metadata includes.
Plain generation does not create, update or delete the bridge entrypoint.
Source resources remain unchanged. Failed builds must not consume stale output.

`cargo check` regenerates without running the application. Rust-analyzer obtains
OUT_DIR through Cargo's build scripts; enable `cargo.buildScripts.enable`.
If a client does not rerun on FTL saves, run cargo check or reload the workspace.
IDE regeneration and hot reload in a running process are separate mechanisms.

## 4. Connect the runtime

The [headless consumer](examples/minimal/src/main.rs) is a compiled example.
Set `AssetPlugin.file_path` to your deployment asset root, then add
`LocalizationPlugin::<Translations>::new(texts::CATALOG_ASSET_PATH)`.
The example anchors paths to its own package for repeatable tests; a deployed
application should choose its own resource policy.

Create Text/Text2d with `LocalizedText::<Translations>::new(...)`. Capture owned
arguments and replace the component when an argument changes. Deferred
`Message<Translations>` can render against `localization.catalog()`; avoid
capturing already translated strings if they should follow language switches.

Use `Localization<Translations>::set_locale` to select a compiled language.
Insert `Localization::new(locale)` before adding the plugin to override startup.
Fonts, glyph coverage, keys, windows and presentation of errors remain app policy.

## 5. Pass named scopes or request resources

For `presentation/hud.ftl`:

```rust
fn draw_presentation(texts: &texts::Presentation) {
    let hud: &texts::presentation::Hud = texts.hud();
    draw_hud(hud);
}

fn draw_hud(texts: &texts::presentation::Hud) {
    // Only this module's methods are available.
}

fn hud_system(texts: bevy::prelude::Res<texts::presentation::Hud>) {
    draw_hud(&texts);
}
```

The root is Translations; folders expose snake_case modules and named group
types. Each file exposes only its PascalCase leaf type: `presentation::Hud`,
not a public `presentation::hud` module. Additional upstream argument/structured
result types are re-exported beside that leaf with a leaf-name prefix, e.g.
`presentation::HudPrompt`. Colliding scope/message type names and a path being
both file and directory are errors. Catalog remains available as a domain name.

The plugin publishes immutable root/group/leaf resources sharing parsed bundles
through Arc. They exist before Startup. Change language through the central
Localization resource, never through a separate per-HUD state.

Publication runs in PreUpdate's `LocalizationSystems::Publish` and before
PostUpdate text refresh in `LocalizationSystems::Refresh`. For Update readers,
switch in PreUpdate before Publish. A switch during Update reaches direct
resources in PostUpdate's Refresh set; order PostUpdate readers with
`.after(LocalizationSystems::Refresh)`. The central state can therefore be newer
than direct resources within that Update. Idle frames, identical reloads and
inactive-language edits do not replace active resources or mark them changed.

## 6. Reload, rejection and recovery

Enable `watch` plus Bevy's watcher. The runtime reads the definition as opaque
bytes, calls the provider's `descriptor`, resolves safe relative asset addresses,
and watches successfully read modules. It does not parse localization.toml.
Named Bevy asset sources are retained for dependent loads.

The bridge compares parsed definition fields with compiled expected values.
Comments, formatting and field order are accepted; changed values, unknown fields
or invalid UTF-8/TOML reject the whole definition until repaired or rebuilt.
Directory aliases and omission are compared by resolved value: an omitted path
and explicit `"."` are equivalent, but removing a configured `"translations"`
changes the path and requires regeneration. Missing or invalid required fields
are still rejected.

Each complete language is validated independently. A bad module rejects all
changes to that language, retaining its last-known-good snapshot, while another
valid language may update. This is not a transaction across separate disk saves.
Generated parsing checks exact key/reference fingerprints and upstream typed/
structured contracts; direct plain parsing uses the same checks.

Observe `CatalogUpdate` after Publish. Rejected with `locale: None` denotes a
definition/load failure; Some(locale) identifies a language candidate.
Text refresh replaces the contents of bound Text/Text2d components in place
without recreating entities. Keep editable drafts separate from these bindings;
the runtime does not manage text-editor state.

A read-but-invalid module stays watched and can recover automatically.
An initially missing file needs `ReloadCatalogs::<Translations>::default()`
after creation. Missing external files leave embedded translations usable.

## 7. Extend the catalog or provide your own adapter

Add a canonical language directory, e.g. pt-BR, with the complete module tree.
Run cargo check and restart; no Rust enum or language list needs editing.
New modules, languages, keys, references, argument contracts and configuration
values require regeneration. Compatible translated prose does not.

For a different source format, use the runtime without `codegen` and implement
`FluentCatalog`. Its `descriptor(&[u8]) -> Result<CatalogDescriptor, String>`
interprets your definition bytes; its `parse` checks complete candidate catalogs.
The runtime continues to own asset loading, active language, publication and text
bindings. Your provider owns its format and compatibility policy.

The companion bridge is nested in this repository at `codegen_bridge/`.
The independent generator uses a versioned dependency, patched by the consuming
workspace to [its Git repository](https://github.com/SDA-31/fluent_typed_codegen)
until crates.io publication. Use the Git setup in the [README](README.md#optional-generation),
or a caller-owned checkout override for development.

## 8. Verify

From a standalone checkout, use the explicit manifests and generator override in
[Verification](README.md#verification). Run both example binaries and test the
runtime, bridge and example packages. After initial dependency resolution,
`--locked --offline` reuses the local lockfiles and cache.

If an enclosing workspace lists the runtime, bridge, example and generator as
members, its lockfile and root override also support these commands:

```sh
cargo run --locked --offline -p localization-example
cargo run --locked --offline -p localization-example -- --watch
cargo test --locked --offline --workspace --all-features
cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
cargo doc --locked --offline -p bevy_fluent_typed -p bevy_fluent_codegen_bridge -p fluent_typed_codegen --all-features --no-deps
```

The example starts in English, exercises EN/ES/RU and exits after external loads.
Watch mode waits for edits until Ctrl+C. Neither mode edits source files.
Use separate feature/consumer checks to verify dependency isolation: a workspace
all-features build intentionally enables combinations consumers may not use.
