# Typed messages in Bevy scenes

Native `bsn!` scenes accept typed `LocalizedText` constructors on Bevy 0.19 and
0.20. Use the [generated setup](https://github.com/SDA-31/bevy_fluent_typed#setup)
and enable scenes on the application's Bevy dependency:

```toml
[dependencies]
bevy = { version = "0.20", features = ["bevy_scene"] }
bevy_fluent_typed = { version = "0.3.0", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.3.0", default-features = false, features = ["build"] }
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
use bevy_fluent_typed::LocalizedText;
use texts::presentation::Hud as Interface;
```

Then create the binding directly beneath its text target:

```rust,ignore
fn spawn_greeting(mut commands: Commands) {
    let name = String::from("Ada");

    commands.spawn_scene(bsn! {
        Text
        LocalizedText::<Interface>::new(move |hud| hud.msg_hello(&name))
    });
}
```

Register it with `.add_systems(Startup, spawn_greeting)`. The same block works
on 0.19 and 0.20.
The native `FromTemplate` implementation supplies Bevy's scene template;
no `template(...)` callback or default localized message is needed. A scene
that omits its message constructor returns an error rather than an empty binding.

Scene construction needs no ready `Res<Interface>`. With default Auto, inserting
the scene's `LocalizedText<Interface>` component requests and retains its scope.
Removing the last binding releases its demand unless another binding or localized
system still needs the module. Constructing a scene or cloning a binding before
insertion is passive. Explicit Full loads every module; explicit Lazy still uses
application-owned `load`/`unload` calls.

## Reuse a message or binding

The same `from` constructor accepts a deferred `Message<Interface>` or an existing
`LocalizedText<Interface>` on both backends. For a stored message:

```rust,ignore
use bevy_fluent_typed::Message;

fn spawn_notice(mut commands: Commands) {
    let message = Message::<Interface>::new(|hud| hud.msg_hello("Ada"));

    commands.spawn_scene(bsn! {
        Text
        LocalizedText::<_>::from(message)
    });
}
```

The scope is inferred from `Message<Interface>`; BSN still needs the `::<_>` placeholder.

For an existing binding, use `LocalizedText::<Interface>::from(binding)`.
Clone it first when several entities need it. Bevy 0.20 also accepts a component
variable directly, such as `Text binding`; on 0.19 use the typed constructor above.
`LocalizedText::clone()` shares its formatter and owned arguments through `Arc`.
Each label follows the current catalog independently, including language changes
and translation reloads. In explicit Lazy, unloading its scope clears its text until that scope
is ready again. Replace a binding when its captured arguments change.

## Localize a text span

Use the same binding on a `TextSpan` child of a `Text` or `Text2d` root:

```rust,ignore
TextSpan
LocalizedText::<Interface>::new(|hud| hud.msg_hello("Ada"))
```

The binding changes only that span's content and does not insert a root `Text`
on the span entity. The application owns its parent `Text`/`Text2d`, fonts,
colors and hierarchy. In Auto, the inserted span binding requests and retains
its scope until removed, exactly like a root text binding. Other consumers or
manual pins can keep that scope alive. Spans keep their active-language content
while a requested locale loads or fails validation. Keep each translated message complete;
do not assemble a sentence from separately translated fragments in Rust.

## Bevy 0.19 dependencies

Select the matching backend on the normal dependency; leave the build dependency
as shown above. The scene constructors stay the same:

```toml
[dependencies]
bevy = { version = "0.19", features = ["bevy_scene"] }
bevy_fluent_typed = { version = "0.3.0", default-features = false, features = ["bevy-0-19", "codegen"] }
```

## World labels and scene setup

Use `Text2d` in place of `Text` for a world label. Cameras, fonts, shaping and
layout remain normal Bevy application setup. A binding without `Text`, `Text2d`
or `TextSpan` also receives a default UI `Text` during refresh.

These recipes use compiled Rust scenes. They do not add a `.bsn` file loader or
serialize formatting closures. Translation reload keeps the generated schema
unchanged; changing messages or argument types requires rebuilding the application.

The [headless BSN example](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/bsn)
contains a runnable file-backed application and tests for late loading, language
changes, reload rejection/recovery, UI/world labels, unloading and missing messages
on both backends.
