# Migrate from bevy_fluent_typed 0.2.2 to 0.3

Runtime 0.3.0 supports stable Bevy 0.20 and requires Rust 1.97.1 or newer.
The generator stays at 0.2.2. Constructor signatures, native resources and
embedded manifest declarations remain available, but the default loading mode
changes from eager Full to consumer-driven Auto.

## Adopt automatic module loading

The unchanged common constructor now returns an Auto plugin:

```rust,ignore
app.add_plugins(LocalizationPlugin::<texts::Translations>::new(texts::manifest()));
```

Inserted `LocalizedText<Scope>` components request and retain their scope until
removed. Required `Res<Scope>` parameters request it when their system is
initialized through `add_localized_systems` or `localized`. No explicit `load`
call is needed. Recurring systems retain demand even when `run_if` is false;
their ownership ends when their system state is dropped.

`add_localized_startup_systems` requests its direct required scopes, runs once
when ready and releases demand after the body returns, including returned
errors. Deferred bindings take over demand without an unloading gap.

No consumers means no module reads or parsing. Plain `add_systems`, optional
resources, `World::get_resource` inspection and unattached messages do not
request modules. Keep explicit Full if your application directly polls the
world or needs a complete language loaded independently of consumers:

```rust,ignore
use bevy_fluent_typed::{Full, Localization, LocalizationPlugin};

app.add_plugins(LocalizationPlugin::<texts::Translations, Full>::new(texts::manifest()));
// Any controller parameters or preinserted controllers must use this mode too:
type AppLocalization = Localization<texts::Translations, Full>;
```

Existing manual Lazy code keeps its explicit mode and `load`/`unload` calls.
`LocalizationPlugin::<texts::Translations>::new_lazy(manifest)` still selects
Lazy. Use the same mode on the plugin and its controller; resource scopes such
as `Res<Hud>` do not change. See [module lifetime](../GUIDE.md#automatic-module-lifetime).

## Use Bevy 0.20

Update both facade dependencies and the application's Bevy dependency:

```toml
[dependencies]
bevy = "0.20"
bevy_fluent_typed = { version = "0.3.0", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.3.0", default-features = false, features = ["build"] }
```

Keep `bevy_fluent_typed::build()` in `build.rs` and your `translations!`
declaration. Standard consumers do not declare the generator separately.
Applications updating other Bevy APIs should consult the
[upstream migration guide](https://bevy.org/learn/migration-guides/0-19-to-0-20/).

## Keep Bevy 0.19 or an older supported backend

The default backend changed. To retain Bevy 0.19, select it explicitly:

```toml
[dependencies]
bevy = "0.19"
bevy_fluent_typed = { version = "0.3.0", default-features = false, features = ["bevy-0-19", "codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.3.0", default-features = false, features = ["build"] }
```

The same pattern supports `bevy-0-16` (minimum 0.16.1), `bevy-0-17` and
`bevy-0-18`. Select exactly one backend on the normal dependency. Keep backend
flags out of the build dependency. `codegen` enables manifest support; handwritten
providers can enable `manifest` explicitly when they need TOML constructors.
Re-enable `watch` if your application uses filesystem notifications.

The new Rust minimum applies to every backend. Stay on runtime 0.2.2 and
generator 0.2.2 if you need Rust 1.95.

## Move from the compatibility preview

Replace Git dependencies on `compat/bevy-0.20` with the 0.3.0 facade entries
above, and replace exact Bevy RC requirements with `bevy = "0.20"`.
The final preview snapshots are preserved as `bevy-0.20.0-rc.1` and
`bevy-0.20.0-rc.2`. The stable backend admits compatible patches.
