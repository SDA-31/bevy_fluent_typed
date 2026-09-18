# Bevy localization integration suite

For minimal starting points use [codegen](../codegen) or
[no_codegen](../no_codegen). This example keeps its larger catalog and watcher
regressions. All generated consumers use an explicit call from build.rs;
`translations!` only includes its output.

This is a complete application with its own build script and modular EN/ES/RU
sources. Code, explanations and the initial greeting are English; Spanish and
Russian remain real translations, not alternate versions of the example code.
No generated Rust is checked in. It runs without a window or GPU.
The runtime dependency is bevy_fluent_typed, deliberately renamed to
`localization_runtime` in Cargo with opt-in `codegen` and `watch` features.
The build-dependency is the same public crate under the same alias, with defaults
disabled and only feature `build`. The bridge is an implementation detail.
This public facade is available since 0.1.1.
The generator resolves from crates.io. Example manifest paths reference packages
inside this repository; external applications use the
[registry setup](../../README.md#optional-generation).
The example and runtime support Bevy 0.16, 0.17, 0.18 and 0.19 (default).
To select an older backend, set `default-features = false` and add `bevy-0-16`,
`bevy-0-17` or `bevy-0-18` to the normal dependency's features in Cargo.toml. Do not change
the build-dependency or forward backend features to both dependency kinds.
Each accepts compatible patches, starting at 0.16.1 for the oldest backend and
.0 for the others. The example imports Bevy through the runtime's re-export so
it cannot accidentally select a second engine minor.
Exact pins are used only in isolated minimum-version verification, not in
library dependency requirements.

From a standalone checkout, use the explicit `examples/minimal/Cargo.toml`
manifest as shown in the
[runtime README](../../README.md#verification). For watch mode, append `-- --watch`.

If an enclosing workspace lists this example as a member, use its generator
override and lockfile:

```sh
cargo run --locked --offline -p localization-example
cargo run --locked --offline -p localization-example -- --watch
```

For the short asset-to-type walkthrough, run `--bin typed_resources` with the
same manifest. Its [source](../../docs/typed_resources.rs) is embedded directly
in the library's Rustdoc landing page. It shows `texts::presentation::Hud`,
chained typed borrows, a direct `Res` parameter and automatic updates to a Bevy
text entity after a language switch. A small launcher supplies this package's
renamed dependency, and a test executes the same source. This example uses the
embedded catalogs immediately and does not wait for external loading.
`localization-example` remains the default binary and the external-load/watch demo.

The default binary waits for successful **external** loads, updates an existing
Bevy `Text`, switches through every discovered language and exits. Watch mode
keeps the default English label alive; edit `assets/localizations/translations/en/ui.ftl`
**inside this example**, not another application's resources. Valid text updates
print to the terminal. Invalid edits print a diagnostic and retain the previous text.
Stop watch mode with Ctrl+C. No example files are modified by either command.

The configuration is `assets/localizations/localization.toml` inside this
example. Both `source-language` and `default-language` are `en`. To watch Spanish
or Russian instead, change `default-language` to `es` or `ru` and rebuild before
starting watch mode. No enum or build-script language list needs editing.
Each language has the same eleven FTL modules; `es/` includes Spanish punctuation
and accents as well as the nested HUD/panel messages.

```text
assets/localizations/
  localization.toml
  translations/
    en/
    es/
    ru/
```

Cargo metadata selects `asset-root = "assets"` and
`catalog = "localizations/localization.toml"`; the TOML selects
`translations-directory = "translations"`. These paths belong to this example,
not to a fixed library-wide resource tree. The directory parameter can be omitted
when locale folders are directly beside the TOML; the default is `"."`.

`src/main.rs` declares `localization_runtime::translations!(pub mod texts)`.
The macro includes Cargo output and resolves the renamed dependency automatically;
there is no handwritten `src/generated.rs` or `src/catalog.rs`. The generator
adapter supplies `FluentCatalog`. The example demonstrates the Bevy plugin and deferred
arguments, and `src/tests.rs` exercises the generated contract.
The example anchors its asset root to its own Cargo package for repeatable runs.
A packaged application should choose its deployment root instead.
The extra `translations/` level deliberately exercises a configured language
directory, rather than assuming languages must be beside the TOML file.

Macro regressions cover renamed dependencies, caller-name shadowing, public and
restricted visibility, nested placement and forwarded module attributes. The
remaining raw includes deliberately exercise low-level/custom frontend support.
Generated-contract regressions cover English startup and discovery of EN/ES/RU,
original module sources, external validation, engine-free output, deferred
arguments and isolation of upstream private locale symbols. The nested
`presentation/hud.ftl` and `presentation/panel.ftl` deliberately repeat keys with
different argument contracts. Tests exercise explicit `Presentation` / `Hud`
types, shared resources (ECS-immutable on 0.19), locale-switch scheduling, local references,
attributes and structured messages. Extra small catalogs exercise Rust keywords
and standard-library-name collisions; they are fixtures, not application features.
The `catalog.ftl` fixture verifies that a domain type named `Catalog` can coexist
with the `Translations` root, including as a Bevy resource.
All three languages run through resource-publication and external-parser checks.
Spanish assertions also verify actual translations, rather than accepting an
English fallback. Independent source/startup language policy remains covered by
the generator and runtime crate fixtures; the public example starts in English.
`src/tests/raw_root.rs` includes the engine-free tree at the crate root, without
a generated wrapper module. These tests are included in workspace verification:

```sh
cargo test --locked --offline -p localization-example
```

`src/tests/reload.rs` edits several copied FTL files through the real filesystem
watcher. It checks UI/world text and direct-resource consistency, rejection of a
broken active language while an inactive language updates, and automatic recovery.
Only disposable temporary assets are edited; example sources remain unchanged.
Separate saves can publish intermediate valid snapshots: this is not an atomic
multi-file editor transaction. Allow up to 15 seconds for watcher convergence.

## Decimal and plural regression

`src/tests/plurals.rs` uses ICU4X DecimalFormatter and PluralRules directly and
passes text/category strings to actual generated `Numbers` accessors. A deferred Bevy
text captures the raw value and per-locale formatters, then follows EN/ES/RU
switches without retaining the old grammar. Separate checks cover visible `1`
versus `1.0` and native Fluent exact numeric matching.

ICU is only a test dependency here, not a dependency of the runtime, bridge or
generator. The separate [ICU resource example](../icu) demonstrates shared Decimal
and percentage formatters, Arabic digits and locale changes. Neither suite renders glyphs or verifies
visual RTL layout. The [guide](../../GUIDE.md#decimal-and-plural-arguments)
explains this ownership split and shows the source-language FTL contract.

## License

[MIT](LICENSE). The example code and translation fixtures are part of the
localization library, not assets from a consuming application.
