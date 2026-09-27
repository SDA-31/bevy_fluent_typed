**Unreleased loading API:** use the matching Git dependencies and generator patch in [the root README](../../README.md#development-setup).

# Minimal example: codegen

One Fluent module, three languages, typed `texts::ui::Greeting` resources.
The program prints English, Spanish and Russian greetings, then exits. No window,
GPU, watcher loop or extra test catalogs are needed.

```sh
localization_generator_git='patch.crates-io.fluent_typed_codegen.git="https://github.com/SDA-31/fluent_typed_codegen"'
localization_generator_rev='patch.crates-io.fluent_typed_codegen.rev="0e5ed447cc08553818a240fc6cfe736aa67fb4b3"'
cargo run --manifest-path examples/codegen/Cargo.toml --config "$localization_generator_git" --config "$localization_generator_rev"
cargo test --manifest-path examples/codegen/Cargo.toml --config "$localization_generator_git" --config "$localization_generator_rev"
```

Run these from the runtime repository. In an enclosing workspace you can also use
`cargo run -p localization-codegen-example`. For Bevy 0.16/0.17/0.18, disable defaults
and add the matching `bevy-0-16` / `bevy-0-17` / `bevy-0-18` to the **normal dependency** features in Cargo.toml.
Leave the build-dependency unchanged.

The complete setup is [Cargo.toml](Cargo.toml), the explicit generation call in
[build.rs](build.rs), [src/main.rs](src/main.rs), and
[assets/localizations](assets/localizations). It needs only the runtime's `codegen`
feature and the **same crate** as an explicit build-dependency, with defaults
disabled and feature `build`. Generation runs only when
build.rs calls `bevy_fluent_typed::build()`. The `translations!` macro
only includes that generated output; it does not scan or write files. The example
uses the public build facade introduced in 0.1.1; the local path tests this checkout.

`assets/localizations/translations/en/ui/greeting.ftl` becomes
`texts::ui::Greeting`. Asset paths and discovery come from the manifest and TOML,
not from a language list in build.rs. The bridge validates sources and explicitly
registers file/directory changes with Cargo. Its generator runs in the host build
graph, not inside the application. See [the guide](../../README.md#development-setup).

For a generator-free provider see [no_codegen](../no_codegen). For deferred
messages, typed arguments and live edits see the larger [integration suite](../minimal).

The application explicitly embeds source bytes and parses the selected language during an update. [tests/output.rs](tests/output.rs)
checks the executable's three greetings; there is no test collection wrapper in main.

[MIT](LICENSE).
