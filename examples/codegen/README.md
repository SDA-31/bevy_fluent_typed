# Minimal example: codegen

One Fluent module, three languages, typed `texts::ui::Greeting` resources.
The program prints English, Spanish and Russian greetings, then exits. No window,
GPU, watcher loop or extra test catalogs are needed.

```sh
cargo run --manifest-path examples/codegen/Cargo.toml
cargo test --manifest-path examples/codegen/Cargo.toml
```

Run these from the runtime repository. In an enclosing workspace you can also use
`cargo run -p localization-codegen-example`. For Bevy 0.17/0.18, disable defaults
and add `bevy-0-17` / `bevy-0-18` to the **normal dependency** features in Cargo.toml.
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
graph, not inside the application. See [the guide](../../GUIDE.md#3-generate-and-index).

For a generator-free provider see [no_codegen](../no_codegen). For deferred
messages, typed arguments and live edits see the larger [integration suite](../minimal).

[MIT](LICENSE).
