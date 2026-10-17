# Typed messages in Bevy scenes

Native `bsn!` scenes accept typed `Localized` constructors on Bevy 0.19 and
0.20. Use the [generated setup](https://github.com/SDA-31/bevy_fluent_typed#setup)
and enable scenes on the application's Bevy dependency:

```toml
[dependencies]
bevy = { version = "0.20", features = ["bevy_scene", "bevy_ui_widgets"] }
bevy_fluent_typed = { version = "0.4.0", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.4.0", default-features = false, features = ["build"] }
```

The localization library adds no scene dependency or separate scene feature.
Use the quickstart's `build.rs`, manifest and generated `texts` module. Add
this message to its HUD file:

```ftl
# $name (String) - Name supplied by the application.
hello = Hello, { $name }!
```

Keep matching messages in every configured language. `DefaultPlugins` installs
Bevy's scene plugin. Applications using `MinimalPlugins` also install
`AssetPlugin` and `bevy::scene::ScenePlugin`; the runnable example does this.

## Create a binding inside BSN

Import the usual component constructor and your catalog scope:

```rust,ignore
use bevy::prelude::*;
use bevy_fluent_typed::Localized;
use texts::presentation::Hud as Interface;
```

Then create the binding directly beneath its text target:

```rust,ignore
fn spawn_greeting(mut commands: Commands) {
    let name = String::from("Ada");

    commands.spawn_scene(bsn! {
        Text
        Localized::<Text, Interface>::new(move |hud| hud.msg_hello(&name))
    });
}
```

Register it with `.add_systems(Startup, spawn_greeting)`. The same block works
on 0.19 and 0.20.
The native `FromTemplate` implementation supplies Bevy's scene template;
no `template(...)` callback or default localized message is needed. A scene
that omits its message constructor returns an error rather than an empty binding.

Scene construction needs no ready `Res<Interface>`. With default Lazy, inserting
the scene's `Localized<Text, Interface>` component requests and retains its scope.
Removing the last binding releases its demand unless another consumer, manual
pin or lease still needs the module. Constructing a scene or cloning a binding before
insertion is passive. Explicit Full loads every module; explicit Manual still uses
application-owned `load`/`unload` calls.

## Reuse a message or binding

The same `from` constructor accepts a deferred `Msg<Interface>` or an existing
`Localized<Text, Interface>` on both backends. For a stored message:

```rust,ignore
use bevy_fluent_typed::Msg;

fn spawn_notice(mut commands: Commands) {
    let message = Msg::<Interface>::new(|hud| hud.msg_hello("Ada"));

    commands.spawn_scene(bsn! {
        Text
        Localized::<Text, _>::from(message)
    });
}
```

The scope is inferred from `Msg<Interface>`; BSN still needs explicit target
and scope parameters: `::<Text, _>`.

For an existing binding, use `Localized::<Text, Interface>::from(binding)`.
Clone it first when several entities need it. Bevy 0.20 also accepts a component
variable directly, such as `Text binding`; on 0.19 use the typed constructor above.
`binding.clone()` shares its formatter and owned arguments through `Arc`.
Each label follows the current catalog independently, including language changes
and translation reloads. In explicit Manual, unloading its scope clears its text until that scope
is ready again. Replace a binding when its captured arguments change.

## Localize a text span

Use `Localized<TextSpan, Scope>` on a `TextSpan` child of a `Text` or `Text2d` root:

```rust,ignore
TextSpan
Localized::<TextSpan, Interface>::new(|hud| hud.msg_hello("Ada"))
```

The binding changes only that span's content and does not insert a root `Text`
on the span entity. The application owns its parent `Text`/`Text2d`, fonts,
colors and hierarchy. In Lazy, the inserted span binding requests and retains
its scope until removed, exactly like a root text binding. Other consumers or
manual pins can keep that scope alive. Spans keep their active-language content
while a requested locale loads or fails validation. Keep each translated message complete;
do not assemble a sentence from separately translated fragments in Rust.

## Bevy 0.19 dependencies

Select the matching backend on the normal dependency; leave the build dependency
as shown above. The scene constructors stay the same:

```toml
[dependencies]
bevy = { version = "0.19", features = ["bevy_scene", "bevy_ui_widgets"] }
bevy_fluent_typed = { version = "0.4.0", default-features = false, features = ["bevy-0-19", "codegen"] }
```

## Buttons and caption slots

A button's visible caption normally lives on a Text child. `@button(...)`
in Bevy 0.20 calls an application scene function; it is not a built-in text
initializer. Accept a deferred Msg and retain it on the actual targets:

```rust,ignore
use bevy::ui_widgets::Button;
use bevy_fluent_typed::{FluentScope, Localized, Msg};

fn button<S: FluentScope>(message: Msg<S>) -> impl Scene {
    let accessible = message.clone();

    bsn! {
        Button
        AccessibleLabel
        Localized::<AccessibleLabel, S>::from(accessible)
        Children [
            Text
            Localized::<Text, S>::from(message)
        ]
    }
}
```

The visible binding belongs to the child, while the semantic name belongs to
its button root. Both use the same message and retain its scope independently.
AccessibleLabel is available on Bevy 0.19/0.20; its binding updates the immutable
component through Bevy hooks after UI accessibility producers. The runtime
leaves the neighboring AccessibilityNode to Bevy; node-only changes are not
independently repaired. No Text is added
on the button root. The application supplies styling, interactions and focus.

Bevy 0.20 uses `@` for scene function calls and `--` between child entities:

```rust,ignore
let message = Msg::<Interface>::new(|hud| hud.msg_hello("Ada"));
let first = message.clone();
let second = message;

commands.spawn_scene(bsn! {
    Node
    Children [
        #FirstButton
        @button(first)
        --
        #SecondButton
        @button(second)
    ]
});
```

On Bevy 0.19, use `button(message.clone())` without `@`, and separate child
entities with commas. Scene IDs such as `#FirstButton` are internal identifiers,
not translation keys.

A reusable widget can instead accept `caption: impl Scene`, keeping its
semantic name as a separate `Msg<S>`. Bevy 0.20 inserts that caption with
`Children [ @caption ]`; Bevy 0.19 uses `Children [ ({caption}) ]`. This slot can
contain text, an icon or a composed scene. Icon-only controls still need an
accessible name even when no visible Text is present.

Feathers uses `FeathersButtonProps.caption: Box<dyn SceneList>` on both versions.
With the application's `bevy_feathers` feature enabled, supply a localized scene
rather than a resolved string:

```rust,ignore
use bevy::feathers::controls::FeathersButtonProps;

let caption_message = message.clone();
let props = FeathersButtonProps {
    caption: Box::new(bsn_list! {
        Text
        Localized::<Text, Interface>::from(caption_message)
    }),
    ..default()
};
```

Attach `AccessibleLabel` and `Localized::<AccessibleLabel, Interface>::from(message)`
to the final button root as well. Feathers remains application-owned; the
localization crate adds no widget/Feathers dependency. String-taking helpers create a translated
snapshot if given a rendered String; keeping a Msg binding inside the
caption preserves language switching.

[Feathers button source](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_feathers/src/controls/button.rs)
and [native BSN migration](https://bevy.org/learn/migration-guides/0-19-to-0-20/#bsn-syntax-improvements).

For application-defined components, see
[custom bindings](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/bindings.md).
`Localized<Target, Scope>` names the target first. Every target implements
`Component + Default`; the engine creates an omitted target once at binding
initialization, including custom components. Later removal is respected.
A binding always needs its formatting closure or Msg.
The runnable BSN example includes typed button and caption-slot helpers and
checks EN/ES/RU visible/semantic labels and release after despawning.

## World labels and scene setup

Use `Localized::<Text2d, Interface>::new(...)` for a world label. Cameras, fonts,
shaping and layout remain normal Bevy application setup. Built-in targets use
`Default` to initialize an omitted component of exactly the selected type once.

These recipes use compiled Rust scenes. They do not add a `.bsn` file loader or
serialize formatting closures. Translation reload keeps the generated schema
unchanged; changing messages or argument types requires rebuilding the application.

The [headless BSN example](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/bsn)
contains a runnable file-backed application and tests for late loading, language
changes, reload rejection/recovery, UI/world labels, unloading and missing messages
on both backends.
