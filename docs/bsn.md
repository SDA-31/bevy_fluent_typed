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

Scene construction needs no ready `Res<Interface>`. Full mode loads the module
automatically. In Lazy mode, the screen owner still calls
`localization.load::<Interface>()` and releases that request with
`localization.unload::<Interface>()` when appropriate. Creating a scene or
cloning a binding never requests translations.

## Reuse a message or binding

The same `from` constructor accepts a deferred `Message<Interface>` or an existing
`LocalizedText<Interface>` on both backends. For a stored message:

```rust,ignore
use bevy_fluent_typed::Message;

fn spawn_notice(mut commands: Commands) {
    let message = Message::<Interface>::new(|hud| hud.msg_hello("Ada"));

    commands.spawn_scene(bsn! {
        Text
        LocalizedText::<Interface>::from(message)
    });
}
```

For an existing binding, use `LocalizedText::<Interface>::from(binding)`.
Clone it first when several entities need it. Bevy 0.20 also accepts a component
variable directly, such as `Text binding`; on 0.19 use the typed constructor above.
`LocalizedText::clone()` shares its formatter and owned arguments through `Arc`.
Each label follows the current catalog independently, including language changes
and translation reloads. Unloading its scope clears its text until that scope
is ready again. Replace a binding when its captured arguments change.

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
layout remain normal Bevy application setup. A binding with neither text
component also receives a default UI `Text` during refresh.

These recipes use compiled Rust scenes. They do not add a `.bsn` file loader or
serialize formatting closures. Translation reload keeps the generated schema
unchanged; changing messages or argument types requires rebuilding the application.

The [headless BSN example](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/bsn)
contains a runnable file-backed application and tests for late loading, language
changes, reload rejection/recovery, UI/world labels, unloading and missing messages
on both backends.
