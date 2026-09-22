# Minimal example: no codegen

No `codegen` feature, no build script, no build-dependencies. The program creates
a localized Bevy `Text`, switches English → Spanish → Russian, prints each
greeting and exits. It runs headlessly, without a window or GPU.

```sh
cargo run --manifest-path examples/no_codegen/Cargo.toml
cargo test --manifest-path examples/no_codegen/Cargo.toml
```

Run from the runtime repository. In an enclosing workspace you can also use
`cargo run -p localization-no-codegen-example`. Select Bevy 0.16/0.17/0.18 with
`--no-default-features --features bevy-0-16` / `bevy-0-17` / `bevy-0-18`.

[src/main.rs](src/main.rs) contains the application;
[src/texts.rs](src/texts.rs) supplies the handwritten `FluentCatalog` required by
the runtime. It uses the runtime's re-export of
[fluent-typed](https://github.com/human-solutions/fluent-typed) to validate and
resolve one argument-free `hello` message. `toml_edit` only reads this provider's
small definition format. Neither the bridge nor generator is needed.

The provider eagerly resolves its entire API before accepting a snapshot. Missing
modules/messages, unresolved references and missing variables are rejected. Extra
unused messages are permitted by this provider; expanding its API requires extending
validation. It is intentionally not a generic replacement for generated contracts.
[tests/provider.rs](tests/provider.rs) validates embedded locales and rejected
external candidates; [tests/output.rs](tests/output.rs) checks the executable's
three greetings. The application itself contains no test assertions.

The example stores text at `assets/localizations/translations/{en,es,ru}/ui/greeting.ftl`.
Its TOML accepts only `translations-directory = "translations"`; the handwritten
provider owns that policy, not the runtime. Locale names are explicit here because
there is no discovery/generator. No standalone module resources are published;
read `Localization<Texts>::catalog()` or use `LocalizedText<Texts>`.

For automatic discovery, typed accessors and module resources use [codegen](../codegen).
[MIT](LICENSE).
