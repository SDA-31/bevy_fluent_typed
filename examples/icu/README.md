# ICU4X formatters as Bevy resources

A headless application that creates Decimal and percentage formatters once per
catalog locale, shares them through a Bevy resource, and injects them into
deferred `LocalizedText` bindings. English documentation and source-language FTL;
English, Spanish, Russian and Arabic translations. No window, fonts or GPU needed.

This example uses the existing `bevy_fluent_typed` API. ICU is an **application
dependency here only**, not a direct dependency or feature of the runtime, generator or
bridge. There is no custom formatter trait to implement.

## Run

From a checkout of this repository:

```sh
cargo run --manifest-path examples/icu/Cargo.toml
cargo test --manifest-path examples/icu/Cargo.toml
```

Use `--locked --offline` after the initial dependency resolution. In the enclosing
development workspace the package is named `localization-icu-example`.
Repository-local paths in Cargo.toml test this checkout; external applications
use the runtime's [versioned dependency setup](../../README.md#optional-generation).
The build script explicitly calls `bevy_fluent_typed::build()`.

Assets are ordinary modular FTL under
`assets/localizations/translations/{en,es,ru,ar}/presentation/hud.ftl`.
The generated tree exposes `texts::presentation::Hud` and
`catalog.presentation().hud().msg_damage(text)`. Both displayed arguments are
annotated `(String)` in English. The percent argument contains its own ICU-owned
symbol and spacing; the FTL adds only the surrounding translated label.

## Follow the source

- [main.rs](src/main.rs) registers the plugin and resource once, then changes
  locales and reads the existing UI/world entities.
- [formatting.rs](src/formatting.rs) owns the resource and binding implementation;
  the library's Rustdoc links here instead of embedding the example's source.
- [tests.rs](src/tests.rs) checks real `Text` / `Text2d` output, exact large
  decimals, visible zeros, ratio scaling, Arabic digits and formatter replacement.

`Res<NumberFormats>` is injected into the spawning system. Bindings capture an
`Arc` of the full locale map and their original numeric values, not a borrowed
`Res`, a preformatted string, or one formatter for the old language. On language
changes or accepted catalog reloads the runtime rerenders using the current
catalog. The closure selects the matching formatter by `catalog.locale()`.
For Arabic the application explicitly selects `ar-EG-u-nu-arab`; the catalog
language remains `ar`. Real applications can configure their own regional and
numbering-system preferences without deriving currency or units from a language.

Changing a value means replacing its binding. Replacing the formatter resource
does **not** update already captured Arcs or notify the runtime automatically.
Rebuild affected bindings when formatter settings change; the regression suite
demonstrates this distinction. Moving an entity does not reformat its number.

## ICU API boundaries

[ICU4X DecimalFormatter](https://docs.rs/icu_decimal/latest/icu_decimal/struct.DecimalFormatter.html)
formats exact `icu_decimal::input::Decimal` values, preserving visible precision.
This input is not `rust_decimal::Decimal` and the example never routes it through
`f64`. Apply any rounding explicitly at the application boundary.

[PercentFormatter](https://docs.rs/icu_experimental/0.6.0/icu_experimental/dimension/percent/formatter/struct.PercentFormatter.html)
is currently experimental. Only this example pins `icu_experimental = "=0.6.0"`;
applications should assess that stability separately. Its input is already in
percent units: `12.5` formats as `12.5%` in English. This application stores a
ratio, so it scales `0.125` by 100 exactly once with `Decimal::multiply_pow10(2)`
and removes shifted leading padding with `trim_start()` (otherwise `012.5%`).
ICU owns the symbol, spacing, separators and digit shapes; no Rust `%` suffix is
appended. The tests also check Turkish prefix placement directly through ICU.

Known limitation of the pinned experimental data: `ar-EG-u-nu-arab` produces
Arabic-Indic digits but an ASCII `%` with left-to-right marks, not the Arabic
percent sign U+066A. The example preserves and tests that actual ICU output; it
does not substitute characters. Applications requiring U+066A should verify a
suitable ICU version/backend rather than assume the numbering preference covers
percentage symbols too.

`icu_decimal/alloc` enables String output; `icu_provider/sync` enables sharing
ICU payloads across Bevy threads. Formatter initialization errors are returned
from startup. Demonstration values are bounded; validate arbitrary input and
scaling/rounding limits according to your application's numeric domain.

The app immediately uses embedded catalogs and does not wait for asynchronous
asset loading. The separate [integration suite](../minimal) tests real filesystem
hot reload. String assertions preserve Fluent isolation but do not prove visual
RTL layout, glyph shaping, fonts or UI mirroring.

## License

[MIT](LICENSE), for this example and its translation fixtures only.
