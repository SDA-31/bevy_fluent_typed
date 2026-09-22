# Typed resources, text bindings and live edits

For the smallest starting points, use [codegen](../codegen) or
[no_codegen](../no_codegen). This larger example shows language-switchable Bevy
text, typed resources and real filesystem hot reload. It runs without a window
or GPU. English, Spanish and Russian catalogs are included.

## Run

From this repository:

```sh
cargo run --manifest-path examples/minimal/Cargo.toml
cargo run --manifest-path examples/minimal/Cargo.toml -- --watch
cargo run --manifest-path examples/minimal/Cargo.toml --bin typed_resources
cargo test --manifest-path examples/minimal/Cargo.toml
```

In an enclosing workspace, the package is `localization-example`; add
`--locked --offline` after resolving dependencies.

## Start here

- [main.rs](src/main.rs): generated `texts`, plugin setup, a deferred
  `LocalizedText` binding and language changes.
- [typed_resources.rs](src/bin/typed_resources.rs): the shorter resource-focused
  walkthrough. It shows `Translations → Presentation → Hud`, a direct
  `Res<texts::presentation::Hud>` parameter and the same text entity after a
  language switch. This binary uses embedded catalogs immediately.
- [console.rs](src/console.rs): the default binary's terminal runner and load
  diagnostics. This is headless application plumbing, not a localization API.
- [build.rs](build.rs): the explicit generation call; `translations!` only
  includes its output.
- [tests/](tests): contract, resource, watcher and formatting regressions. Start
  with [tests/README.md](tests/README.md) when looking for verification.

The default binary waits for successful **external** loads, prints the bound
text, switches through every discovered language and exits. Watch mode keeps the
English label alive: edit `assets/localizations/translations/en/ui.ftl` **inside
this example**. Valid edits print to the terminal; invalid edits print a diagnostic
and retain the previous text. Stop watch mode with Ctrl+C. Neither command edits
the example's files.

## Configuration

[Cargo.toml](Cargo.toml) deliberately renames `bevy_fluent_typed` to
`localization_runtime` in both dependency sections. The normal dependency enables
`codegen` and `watch`; the same crate's build-dependency disables defaults and
enables only `build`. The bridge is an implementation detail. This public build
facade is available since 0.1.1. Repository-local paths test this checkout;
external applications use the [registry setup](../../README.md#optional-generation).

The normal dependency selects Bevy 0.19 by default. To use 0.16, 0.17 or 0.18,
disable its defaults and select the corresponding backend feature; leave the
build-dependency unchanged. Compatible patches are accepted, starting at 0.16.1
for the oldest backend and .0 for the others.

Cargo metadata selects `asset-root = "assets"` and
`catalog = "localizations/localization.toml"`. That TOML selects
`translations-directory = "translations"`; source and default languages are
`en`. To watch another language, change `default-language` and rebuild.

The example anchors its asset root to its own package for repeatable runs.
A packaged application should choose its deployment root instead. These paths
are example configuration, not fixed library conventions.

The extra catalogs with names such as `type`, `vec` and `as-ref` belong to the
[test suite](tests/README.md): they exercise generated-name collisions, not
additional application setup. The separate [ICU example](../icu) demonstrates
application-owned Decimal and percentage formatters through Bevy resources.

[MIT](LICENSE), for the example and its translation fixtures only.
