# Migrate from bevy_fluent_typed 0.3.0 to 0.4.0

[Migration Guide](README.md)

Also apply the [explicit target and Msg migration](migration-0.3.1.md) when replacing older text bindings.

Runtime 0.4.0 keeps the loading modes, provider contract and generator 0.2.2.
On Bevy 0.19/0.20, catalog consumers now use ordinary native system registration.
Rust 1.97.1 remains the minimum. Bevy 0.16–0.18 remain supported, with
their existing registration helpers for direct catalog demand and readiness.

## Register required catalog consumers normally

Before:

```rust,ignore
use bevy_fluent_typed::{LocalizationAppExt, localized};

app.add_localized_systems(Update, update_hud);
app.add_systems(Update, localized(update_buttons).run_if(screen_is_open));
app.add_localized_startup_systems(setup_hud);
```

After, on Bevy 0.19/0.20:

```rust,ignore
app.add_systems(Update, update_hud);
app.add_systems(Update, update_buttons.run_if(screen_is_open));
app.add_systems(Startup, setup_hud);
```

Keep native `Res<Scope>` parameters. The plugin discovers aliases, derived
`SystemParam`s, nested parameters, `ParamSet` and catalog reads in run conditions.
In Lazy, recurring consumers own their scopes for their system-state lifetime,
even when an application condition is false. Full and Manual retain their existing
loading policies. Install localization plugins before schedule initialization;
already initialized systems retain their previous behavior. Custom schedules need
an App update to discover them before their first manual run.

`LocalizationAppExt`, `localized` and `IntoLocalizedSystems` are removed on
Bevy 0.19/0.20, including their independent Update-based startup helper. Replace
their imports and registrations on those backends. On Bevy 0.16–0.18 the helpers
remain available: direct required `Res<Scope>` parameters request catalogs in Lazy
and wait for readiness. `add_localized_startup_systems` runs once in Update; it
does not preserve native startup phases or original function ordering. See the
[older-backend recipe](../../GUIDE.md#older-bevy-backends).
`Localized` still owns its scope automatically in Lazy on every backend.

## Account for deferred startup

The default startup policy changes to **Deferred** on Bevy 0.19/0.20.
Independent steps execute once; catalog consumers and their declared dependents
wait. `PostStartup` follows completed Startup. Original function ordering, sets
and deferred commands remain available. Completed steps do not rerun after locale
changes; insert `Localized` for live text.

Frame schedules continue while startup waits. Guard frame systems that need
non-catalog resources produced by unfinished startup steps, and initialize asset
sources/loaders before their catalog consumers. Failed catalog loads remain pending
until explicitly retried. OnEnter and custom one-shot schedules do not gain retries.

Configure the App's startup policy through `LocalizationStartupPlugin`.
Choose **Native** to retain ordinary one-shot startup:

```rust,ignore
use bevy_fluent_typed::{LocalizationStartupPlugin, StartupStrategy};

app.add_plugins(LocalizationStartupPlugin::new(StartupStrategy::Native));
```

Native does not automatically request or await startup catalog reads.
Use already-ready catalogs or startup code that only inserts bindings; ordinary
recurring consumers and inserted bindings remain automatic in Lazy.
For an explicit group that waits for all its catalog requirements, instead add
`LocalizationStartupPlugin::barrier(AppStartup)` and place its systems in that
`SystemSet`. See [startup strategies](../../GUIDE.md#startup-strategies).

Register the startup plugin before or after the root plugin, before the first
App update. An absent startup plugin is added as an implicit Deferred fallback;
an explicit plugin added later selects the policy without a second coordinator.
All root plugins inherit the App's policy. Matching explicit startup plugins
are accepted; contradictory choices and changes after startup begins are rejected.

## Keep optional observers passive when needed

Bevy exposes identical scheduler read metadata for `Res<Scope>` and
`Option<Res<Scope>>`; ordinary optional reads are therefore conservatively treated
as consumers on Bevy 0.19/0.20. An observer that must run while absent can use
`Option<PassiveCatalog<Scope>>`, which immediately receives `None` without requesting
or retaining the module. Broad World inspection stays passive. On Bevy 0.16–0.18,
native `Option<Res<Scope>>` remains passive because there is no resource-read discovery.

## Update both dependency entries

```toml
[dependencies]
bevy = "0.20"
bevy_fluent_typed = { version = "0.4.0", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.4.0", default-features = false, features = ["build"] }
```

To retain Bevy 0.19, disable normal-dependency defaults and enable `bevy-0-19`
alongside `codegen`. Leave the build-only dependency unchanged. The default
Deferred policy needs no explicit startup plugin or new generator dependency.
