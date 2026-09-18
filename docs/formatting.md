## Injecting ICU formatters through Bevy resources

Number formatting is application work. Use dedicated
[ICU4X](https://docs.rs/icu/) or
[ICU](https://unicode-org.github.io/icu/userguide/format_parse/) services for
numbers, percentages, currencies and dates. Neither this runtime nor its codegen
bridge adds an ICU dependency or a formatting feature.

The following code creates reusable services per catalog locale and captures a
shared map in deferred bindings. The system receives it through `Res`; closures
select a formatter from the current catalog, preserving language changes without
rebuilding the UI. The complete [runnable example](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/icu)
compiles this exact source and tests existing Bevy UI/world text.

The example's dependencies, in addition to the standard generated consumer setup:

```toml
[dependencies]
icu_decimal = { version = "2.3", features = ["alloc"] }
icu_experimental = "=0.6.0"
icu_locale_core = "2.3"
icu_provider = { version = "2.3", features = ["sync"] }
writeable = "0.6"
```

`PercentFormatter` is experimental and pinned **only in the example**. Its input
is in percent units; the application converts a ratio such as `0.125` to `12.5`
once before formatting. ICU supplies the percent symbol and its locale-specific
placement; Fluent receives the complete formatted String. DecimalFormatter and
the percentage formatter reuse the same configured decimal formatting data.
Scaling also shifts visible leading padding, so the example uses ICU Decimal's
`trim_start()` after multiplying. In the pinned experimental version, the chosen
Arabic locale yields Arabic-Indic digits but ASCII `%` with bidi marks, not U+066A.
The example preserves and tests that output; consumers needing U+066A must assess
their ICU version/backend. No manual symbol substitution is implemented.

The generated `texts` module needs `presentation/hud.ftl` in EN/ES/RU/AR, with
these English source contracts:

```ftl
# $value (String) - Locale-formatted damage.
damage = Damage: { $value }
# $value (String) - Complete localized percentage, including symbol and spacing.
chance = Chance: { $value }
```

Call `app.insert_resource(NumberFormats::try_new(GroupingStrategy::Auto)?)`
once during setup and register `spawn_labels` in `Startup`, after configuring
Bevy and [`LocalizationPlugin`]. The standalone example handles that wiring.
Generated types and these optional application dependencies are not available
inside this library's own doctest environment; the example compiles the snippet.

Locale switches rerender the existing bindings, selecting a prebuilt formatter;
they do not recreate services. A change to an arbitrary resource is not an
automatic refresh signal. Replacing `NumberFormats` leaves previously captured
Arcs intact: replace affected bindings when formatting policy or raw values
change. The example has a regression test for that lifecycle. Arbitrary input
scaling/rounding limits remain application policy; the demonstration inputs are
bounded. Arabic string tests do not verify visual RTL layout or font coverage.
