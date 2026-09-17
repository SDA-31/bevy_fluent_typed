# Guide: typed localization with Bevy

The [README](README.md#optional-generation) contains the current dependency setup.
This guide covers the directory-generator integration; custom providers can use
the runtime without its optional `codegen` feature.

For the smallest starting points, see [codegen](examples/codegen) and
[no_codegen](examples/no_codegen). The larger [integration suite](examples/minimal)
is intended for typed arguments, watching and regression tests.

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
as a String, with a separate selector if grammar requires it.

### Decimal and plural arguments

Typed accessors and Fluent resolution come from
[fluent-typed](https://github.com/human-solutions/fluent-typed). For Decimal-based
formatting, add [fluent_typed_decimal](https://github.com/SDA-31/fluent_typed_decimal)
as an application dependency, not a build-dependency. It does not replace Fluent
or add a numeric type to its syntax.

In a source-language `numbers.ftl`:

```ftl
# $value (String) - Locale-formatted number text.
# $plural (String) - ICU plural category of the same visible value.
remaining = { $plural ->
    [one] { $value } item left
   *[other] { $value } items left
    }
```

`formatter.localize(&decimal, PluralRuleType::Cardinal)` returns our adapter's
`LocalizedNumber`. The generated call is
`catalog.numbers().msg_remaining(number.selector(), number.text())` because the
source pattern encounters `plural` first. ICU chooses the category after applying
the display precision; Fluent then matches the String literally. Other languages
may add `[few]`, `[many]`, `[two]` or `[zero]` as their grammar requires. Keep a
starred fallback; unknown strings go there. Decimal formatting does not make
the String selector match numeric `[0]` or `[1]` variants.

Alternatively, keep a native Fluent Number selector alongside formatted String
text. Both APIs remain available. Required application states (for example an
empty inventory) can choose a separate message from the raw value, independently
of rounding and grammar. Never infer a number back from its localized text.

For `Message` / `LocalizedText`, keep the Decimal and reusable formatters owned
by the closure, select a formatter using the current catalog's `locale()`, then
prepare the two strings. Capturing an already-localized number would preserve
the old language after a switch. The
[compiled regression](examples/minimal/src/tests/plurals.rs) checks the actual
Bevy text after each switch. Its input is bounded test data; applications should
handle adapter errors according to their numeric-domain policy.

The adapter's own tests cover Arabic categories, fractional values and `arab` /
`latn` digits. Bidi isolation comes from Fluent interpolation, not this adapter.
Visual RTL ordering, Arabic shaping, font coverage and mirrored UI remain renderer
responsibilities; the headless examples do not claim to test those.

## 3. Generate and index

Enable `bevy_fluent_typed/codegen` at runtime and use
that same crate with defaults disabled and `build` in build-dependencies.
The README's registry setup uses the public facade introduced in **0.1.1**.
Return its result:

```rust
fn main() -> std::process::ExitCode {
    bevy_fluent_typed::build()
}
```

The internal bridge is not named by the application. Build-only use needs no Bevy
backend. `build()` emits readable errors without panicking. `from_cargo()` returns Result for
custom handlers. Do not ignore generation errors: output is not transactional.
Generation belongs to this explicit build call, not macro expansion. The macro
below only includes prepared Cargo output. The build-dependency remains explicit
because a normal dependency feature cannot install consumer build dependencies.

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

The plugin publishes root/group/leaf resources sharing read-only parsed bundles
through Arc. Bevy 0.19 also enforces ECS resource immutability; 0.16–0.18 do not.
Use `Res` for catalog modules on every backend. They exist before Startup. Change language through the central
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
The independent generator uses a versioned crates.io dependency. Use the setup
in the [README](README.md#optional-generation), or a caller-owned checkout
override for generator development.

## 8. Verify

From a standalone checkout, use the explicit manifests in
[Verification](README.md#verification). Run both example binaries and test the
runtime, bridge and example packages. After initial dependency resolution,
`--locked --offline` reuses the local lockfiles and cache.

If an enclosing workspace lists the runtime, bridge, example and generator as
members, its lockfile and root override also support these commands:

```sh
cargo run --locked --offline -p localization-example
cargo run --locked --offline -p localization-example -- --watch
cargo test --locked --offline --workspace
cargo clippy --locked --offline --workspace --all-targets -- -D warnings
cargo doc --locked --offline -p bevy_fluent_typed -p bevy_fluent_codegen_bridge -p fluent_typed_codegen --features bevy_fluent_typed/codegen,bevy_fluent_typed/watch,bevy_fluent_codegen_bridge/build --no-deps
```

The example starts in English, exercises EN/ES/RU and exits after external loads.
Watch mode waits for edits until Ctrl+C. Neither mode edits source files.
Use separate feature/consumer checks to verify dependency isolation. The runtime
requires exactly one `bevy-0-16`, `bevy-0-17`, `bevy-0-18` or `bevy-0-19` backend; the last is
the default. Disable defaults to select an older backend, and match the engine
minor in the application's own dependencies. The build-only facade disables
defaults and needs no backend flag. Select backend flags directly in the normal
dependency; do not forward them to the shared name through consumer features.
Do not use `--all-features` on the runtime or an enclosing workspace.

The [maintainer compatibility command](tools/compatibility/README.md) tests exact
engine releases in isolated Cargo graphs, including watcher reloads and the
version-dependent resource mutability contract. It is a Rust-only development
tool, not part of the runtime or generation dependency graph.
