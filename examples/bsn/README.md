# Typed localization in BSN scenes

The headless application creates a Bevy scene before translations are ready,
loads its generated HUD from files, prints `Hello, Ada!` and exits. No window,
camera or font asset is needed. It targets Bevy 0.20 and demonstrates a scene
factory that also works on 0.19.

## Run

From this repository:

```sh
cargo run --manifest-path examples/bsn/Cargo.toml
cargo test --manifest-path examples/bsn/Cargo.toml
```

These commands use generator 0.2.2 from crates.io and the runtime from this
checkout. For changes to both libraries, use the
[local generator override](../../docs/build.md#work-on-local-checkouts).
For application dependencies and the native syntax for each backend, see the
[BSN guide](../../docs/bsn.md).

## Start here

1. [Cargo.toml](Cargo.toml) enables `bevy_scene` on the application's Bevy
   dependency and uses the normal `codegen`/build-only `build` split.
2. [build.rs](build.rs) explicitly generates `texts::presentation::Hud`.
3. [scene.rs](src/scene.rs) captures an owned name in a `Localized<Text, Interface>`
   and returns a native `bsn!` scene with an explicit `Text` target. `Interface` is an ordinary imported alias.
4. [app.rs](src/app.rs) installs the asset, scene and localization plugins. Lazy
   loading uses the generated manifest helper.
5. [main.rs](src/main.rs) spawns the scene and prints its label after the required
   `Res<Interface>` is ready and text refresh has run.

[buttons.rs](src/buttons.rs) demonstrates Msg-taking buttons: visible text
lives on a child, while an explicit `AccessibleLabel` and its binding supply the
root's semantic name. Caption slots accept a Scene or boxed SceneList, including an
icon-only caption. [Button tests](src/tests/buttons.rs) check EN/ES/RU names,
native scene-call/list syntax and scope release after removing the controls.

Creating a scene or cloning an unattached binding is passive. Inserting its
`Localized<Text, Interface>` requests and retains the scope in default Lazy.
Removing the last binding releases that demand unless another consumer, manual
pin or lease remains.
Explicit Manual still requires manual requests. Retain and clone `Msg` to reuse
its formatter and arguments; construct each `Localized` where it is attached.
Each inserted label follows the current translations independently.

## Verification

[tests.rs](src/tests.rs) uses a controllable byte source to test late requests,
all three locales, unload/reload, invalid translation rejection and recovery,
independent scene-instance arguments, inline message input and rejection of
scenes that omit their formatter. The runnable application reads files;
it does not keep a test byte map in memory.

[Locale tests](src/tests/locales.rs) verify fixed languages beside following
labels, cloned binding conversion, replacement from a retained message,
switching, reload and release, including
Text2d, TextSpan and independent AccessibleLabel selection. The native syntax
helpers cover a prepared binding through `Localized::from` on Bevy 0.19 and
inline `with_locale` chains on 0.20. See the
[several-language guide](../../docs/multilocale.md) for both forms.

[The process test](tests/output.rs) checks the rendered output and bounds the
headless runner with a test-only timeout.

The [compatibility runner](../../tools/compatibility/README.md) tests the same
example against exact Bevy 0.19.0 and 0.20.0. It selects the matching scene test
syntax: inline typed `new`/`from` constructors on both backends and
direct component variables on 0.20. Backend
selection is made on the normal dependency, never forwarded to the host build.

These are compiled Rust scenes, not external `.bsn` file assets.

[MIT](LICENSE).
