# Minimal example: codegen

One Fluent module, three languages, typed `texts::ui::Greeting` resources.
The program prints English, Spanish and Russian greetings, then exits. No window,
GPU, watcher loop or extra test catalogs are needed.

These commands use generator 0.2.2 from the registry. When editing both libraries,
configure the [local generator override](../../docs/build.md#work-on-local-checkouts).
For a new application using this API, follow the [root quickstart](../../README.md#setup).

```sh
cargo run --manifest-path examples/codegen/Cargo.toml --bin localization-codegen-example
cargo test --manifest-path examples/codegen/Cargo.toml
```

Run these from the runtime repository. In an enclosing workspace you can also use
`cargo run -p localization-codegen-example --bin localization-codegen-example`. For Bevy 0.16/0.17/0.18/0.19,
disable defaults and add the matching `bevy-0-16` / `bevy-0-17` / `bevy-0-18` /
`bevy-0-19` to the **normal dependency** features in Cargo.toml.
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
not from a language list in build.rs. The build facade validates sources and explicitly
registers file/directory changes with Cargo. Its generator runs in the host build
graph, not inside the application. See [the guide](../../README.md#setup).

For a generator-free provider see [no_codegen](../no_codegen). For deferred
messages, typed arguments and live edits see the larger [integration suite](../minimal).

The application declares `const EMBEDDED = Translations;` inside
`texts::embed_manifest!`, passes that manifest to the plugin and parses the
selected language when its inserted `LocalizedText<Greeting>` needs it during an update. [tests/output.rs](tests/output.rs) checks the executable's three greetings; there is no test collection wrapper in main.

[MIT](LICENSE).

## Supply your own bytes

Keep the same generated types and build script:

```sh
cargo run --manifest-path examples/codegen/Cargo.toml --bin bytes
```

[src/bin/bytes.rs](src/bin/bytes.rs) passes `(Locale, Greeting::PATH, bytes)` tuples
to `LocalizationPlugin::from_bytes` and prints the same three greetings using
ordinary generated resources. It deliberately embeds its small input with
`include_bytes!`; no runtime manifest or AssetPlugin is required. The buffers
are retained for repeat loads. Default Lazy requests the greeting scope through
the inserted `LocalizedText<Greeting>` binding and keeps it ready while that
binding exists. World inspection alone does not create demand. For on-demand
file reading and module lifetime, use [Lazy](../../GUIDE.md#automatic-module-lifetime)
and Bevy's asset system;
custom storage belongs in an [asset source](../../docs/asset-sources.md).

## Observe native loading progress

The separate file-backed demonstration adds one root
`LocalizationProgressPlugin`, automatically providing views for its generated
UI group and greeting leaf. Its normal dependency still enables only `codegen`,
with the same build-only dependency:

```sh
cargo run --manifest-path examples/codegen/Cargo.toml --bin progress
```

[progress.rs](src/bin/progress.rs) reads native
`Res<LocalizationProgress<texts::ui::Greeting>>` for its change-gated presentation
observer, while its runner reads the root view. Both share one provider dispatcher.
The example orders PostUpdate observation after `LocalizationSystems::Progress`.
Root counts follow current demand; leaf counts describe its fixed unique schema,
including unrequested leaves. Preparation readiness is provider-wide in both
views because a locale commit affects the provider, not one leaf.
This advanced demonstration selects explicit Full for eager loading, prepares
Spanish while English
remains usable, explicitly commits when preparation is ready, and verifies that
idle frames do not update the observer. Counts describe modules, not downloaded
bytes; progress is the latest snapshot, not a notification history.

The generated manifest keeps its `assets/localizations/localization.toml` origin;
this runner sets AssetPlugin's source root to the example's package directory.
[tests/progress.rs](tests/progress.rs) uses controlled asynchronous byte loads to
verify pending/ready snapshots, manual commit, target failure/cancellation,
quiet change detection and group-only recursive selection with the same generated
provider. The group test includes its aliased greeting leaf, excludes the root
and starts no translation I/O. No Cargo progress feature
is needed. Without a progress plugin, ordinary examples have no native progress resource
or recurring tracker. Passive count queries and optional `diagnostics` do not
activate it. See [loading progress](../../GUIDE.md#observe-loading-progress) for your
application's UI and [optional module details](../../GUIDE.md#optional-module-details)
when individual paths or attempt errors are needed.
