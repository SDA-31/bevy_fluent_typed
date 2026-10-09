# Typed messages in Bevy scenes

BSN scenes can contain the same `LocalizedText<Scope>` components as ordinary
entities. Native `bsn!` is available in Bevy 0.19 and 0.20. This guide uses the
[generated setup](https://github.com/SDA-31/bevy_fluent_typed#setup) and runtime
0.3.0's cloneable bindings.

## Enable Bevy scenes

Enable scenes on the application's normal Bevy dependency. The localization
library adds no scene dependency or separate scene feature:

```toml
[dependencies]
bevy = { version = "0.20", features = ["bevy_scene"] }
bevy_fluent_typed = { version = "0.3.0", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.3.0", default-features = false, features = ["build"] }
```

Use the quickstart's `build.rs`, manifest and generated `texts` module. Replace
its HUD message with:

```ftl
# $name (String) - Name supplied by the application.
hello = Hello, { $name }!
```

Keep matching messages in every configured language. `DefaultPlugins` installs
Bevy's scene plugin. Applications using `MinimalPlugins` also install
`AssetPlugin` and `bevy::scene::ScenePlugin`; the runnable example does this.

## Bevy 0.20

A binding is a component value. Pass it directly into `bsn!` with its text target:

```rust,ignore
use bevy::prelude::*;
use bevy_fluent_typed::LocalizedText;
use texts::presentation::Hud as Interface;

fn spawn_greeting(mut commands: Commands) {
    let name = String::from("Ada");
    let greeting = LocalizedText::<Interface>::new(move |hud| hud.msg_hello(&name));

    commands.spawn_scene(bsn! {
        Text
        greeting
    });
}
```

Register `spawn_greeting` in `Startup`. Scene construction needs no ready
`Res<Interface>`: the binding starts empty and refreshes when its module loads.
Full mode loads it automatically. In Lazy mode, the screen owner still calls
`localization.load::<Interface>()` and releases the request with
`localization.unload::<Interface>()` when appropriate. Spawning a scene or
cloning a binding never requests translations.

## Bevy 0.19

Select the matching backend on the normal dependency; leave the build dependency
as shown above:

```toml
[dependencies]
bevy = { version = "0.19", features = ["bevy_scene"] }
bevy_fluent_typed = { version = "0.3.0", default-features = false, features = ["bevy-0-19", "codegen"] }
```

Bevy 0.19's scene-value path requires `Default`. A localized binding has no
meaningful default message. Use Bevy's `template` factory instead:

```rust,ignore
use bevy::{ecs::template::template, prelude::*};
use bevy_fluent_typed::LocalizedText;
use texts::presentation::Hud as Interface;

fn greeting_scene(name: String) -> impl Scene {
    let greeting = LocalizedText::<Interface>::new(move |hud| hud.msg_hello(&name));

    bsn! {
        Text
        template(move |_| Ok(greeting.clone()))
    }
}
```

Then spawn the factory's result:

```rust,ignore
fn spawn_greeting(mut commands: Commands) {
    commands.spawn_scene(greeting_scene("Ada".into()));
}
```

This factory also works on 0.20, so shared code needs no version-specific scene
body. Do not use `template_value(greeting)` on 0.19: it still requires `Default`.

## Reuse and updates

`LocalizedText::clone()` shares the formatter and its owned arguments through
`Arc`. It captures no translation or catalog snapshot. Use clones on separate
entities, or call a scene factory for each instance's arguments. Each binding
updates its own `Text` or `Text2d` when translations reload or the language changes.
Unloading its scope clears its text until the scope is ready again. Replace a
binding when its captured arguments change.

Use `Text2d` in the scene for a world label. Cameras, fonts, shaping and layout
remain normal Bevy application setup. These recipes use compiled Rust `bsn!`
scenes; they do not add a `.bsn` file loader or serialize formatting closures.
A binding with neither text component also receives a default UI `Text` during refresh.

Translation reload keeps the generated schema unchanged. Changing messages or
argument types requires rebuilding the application.

The [headless BSN example](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/bsn)
contains a runnable file-backed application and tests for late loading, language
changes, reload rejection/recovery, UI/world labels and unloading on both backends.
