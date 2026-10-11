# Migrate from bevy_fluent_typed 0.3.0 to 0.3.1

Runtime 0.3.1 keeps the loading modes, provider contract and generator 0.2.2.
On Bevy 0.19/0.20, catalog consumers now use ordinary native system registration.
Rust 1.97.1 remains the minimum; Bevy 0.16–0.18 keep their existing helpers.

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

The old helpers remain available. They infer direct parameters and retain their
original independent Update-based initialization semantics, so existing code does
not have to change all at once.

## Account for deferred startup

The default startup policy changes to **Deferred** on Bevy 0.19/0.20.
Independent steps execute once; catalog consumers and their declared dependents
wait. `PostStartup` follows completed Startup. Original function ordering, sets
and deferred commands remain available. Completed steps do not rerun after locale
changes; insert `LocalizedText` for live text.

Frame schedules continue while startup waits. Guard frame systems that need
non-catalog resources produced by unfinished startup steps, and initialize asset
sources/loaders before their catalog consumers. Failed catalog loads remain pending
until explicitly retried. OnEnter and custom one-shot schedules do not gain retries.

Choose **Native** on the existing plugin to retain ordinary one-shot startup:

```rust,ignore
use bevy_fluent_typed::StartupStrategy;

app.add_plugins(
    LocalizationPlugin::<texts::Translations>::new(texts::manifest())
        .with_startup_strategy(StartupStrategy::Native),
);
```

Native does not automatically request or await startup catalog reads.
Use already-ready catalogs or startup code that only inserts bindings; ordinary
recurring consumers and inserted bindings remain automatic in Lazy.
For an explicit group that waits for all its catalog requirements, use
`.with_startup_barrier(AppStartup)` and place its systems in that `SystemSet`.
See [startup strategies](../GUIDE.md#startup-strategies).

The startup policy is shared by all root plugins in an App. Unconfigured roots
inherit an explicit choice regardless of registration order; conflicting explicit
choices are rejected. Without a choice, the App uses Deferred.

## Keep optional observers passive when needed

Bevy exposes identical scheduler read metadata for `Res<Scope>` and
`Option<Res<Scope>>`; ordinary optional reads are therefore conservatively treated
as consumers on Bevy 0.19/0.20. An observer that must run while absent can use
`Option<PassiveRes<Scope>>`, which immediately receives `None` without requesting
or retaining the module. Broad World inspection stays passive. Optional parameters
inside the legacy helpers retain their earlier behavior.

## Update both dependency entries

```toml
[dependencies]
bevy = "0.20"
bevy_fluent_typed = { version = "0.3.1", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.3.1", default-features = false, features = ["build"] }
```

To retain Bevy 0.19, disable normal-dependency defaults and enable `bevy-0-19`
alongside `codegen`. Leave the build-only dependency unchanged. No second plugin
or new generator dependency is needed.
