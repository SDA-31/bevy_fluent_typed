# Migrate from bevy_fluent_typed 0.3.1 to 0.4.0

[All migration guides](README.md)

This migration replaces implicit text targets with explicit bindings. Loading
modes, generated catalogs, startup strategies, supported Bevy families and the
Rust 1.97.1 minimum remain unchanged. The generator stays at 0.2.2.
Startup configuration now belongs exclusively to `LocalizationStartupPlugin`.
The [changelog](../../CHANGELOG.md#040---2026-10-20) records this version's release status.

## Update both dependency entries

```toml
[dependencies]
bevy = "0.20"
bevy_fluent_typed = { version = "0.4.0", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.4.0", default-features = false, features = ["build"] }
```

Select another Bevy backend on the normal dependency with defaults disabled,
as in the [compatibility recipe](../../README.md#supported-engines). Do not forward
backend features to the build dependency. The existing explicit `build.rs`
generation call and `translations!` declaration do not change.

## Configure startup through the application plugin

On Bevy 0.19/0.20, replace the removed startup builders on `LocalizationPlugin`
with the App-wide `LocalizationStartupPlugin`. Source plugins inherit its policy.

Before:

```rust,ignore
app.add_plugins(
    LocalizationPlugin::<texts::Translations>::new(texts::manifest())
        .with_startup_strategy(StartupStrategy::Native),
);
```

After:

```rust,ignore
use bevy_fluent_typed::{LocalizationPlugin, LocalizationStartupPlugin, StartupStrategy};

app.add_plugins(LocalizationPlugin::<texts::Translations>::new(texts::manifest()));
app.add_plugins(LocalizationStartupPlugin::new(StartupStrategy::Native));
```

Replace `.with_startup_barrier(AppStartup)` with
`app.add_plugins(LocalizationStartupPlugin::barrier(AppStartup))` instead.
Keep the selected systems in `.in_set(AppStartup)`.

The startup plugin can precede or follow source plugins, before the first App
update. With no explicit startup plugin, the existing Deferred fallback remains.
All roots share one coordinator. Matching explicit startup plugins are accepted;
contradictory choices and changes after startup begins are rejected. Loading modes
and execution policies are unchanged. See [startup strategies](../runtime.md#startup-strategies).

## Check duplicate catalog registration during setup

The one-plugin-per-root rule remains. A repeated root now produces a detailed
structural diagnostic through ordinary `add_plugins`, including when both
plugins have the same type. The diagnostic includes the root name, all declared
FTL leaf paths, plugin types and constructor locations. Sources, languages and
translation bytes are not compared; disjoint source subsets still claim the same
complete root.

For a typed `Result` before Bevy registration, call
`plugin.validate_registration(&app)?` and handle `LocalizationRegistrationError`.
This check does not reserve or install anything. `add_plugins` repeats it before
installing the second provider's localization resources or systems. See
[registration conflicts](../runtime.md#registration-conflicts).

## Select the text target explicitly

Before:

```rust,ignore
commands.spawn((
    Text::default(),
    LocalizedText::<texts::ui::Greeting>::new(|catalog| catalog.msg_hello()),
));
```

After:

```rust,ignore
use bevy::prelude::Text;
use bevy_fluent_typed::Localized;

commands.spawn((
    Text::default(),
    Localized::<Text, texts::ui::Greeting>::new(|catalog| catalog.msg_hello()),
));
```

Use `Localized<Text2d, Scope>` for a world-text root and
`Localized<TextSpan, Scope>` for a span. Keep the target component, hierarchy
and styles in the spawn bundle. The old `LocalizedText` type is removed;
the engine no longer infers the destination from neighboring text components.

All targets must implement `Component`; `Default` is not required. Supply each
target alongside its binding. The old implicit UI Text fallback is removed:
if the target is absent when the deferred add callback runs, the binding is
removed with a warning naming the entity, target type and scope. No component is
created and rejected bindings retain no demand or formatter captures.

Supplied settings are preserved. Removing a target after a successful attachment
leaves it absent even after message or locale changes. Reinsert the target to
resume. Removing and adding the binding checks presence again; a previously
rejected binding must be added again after supplying its target.

Removing only the binding stops automatic writes and releases its scope demand;
the target keeps its last value. Other bindings and resource consumers can
still retain the scope. Targets without Default are supported; non-component
field markers are not. Implement `LocalizationTarget` directly on the component.

Ordinary edits to an existing bound Text, Text2d or TextSpan are now restored to
the translation, even when Msg and catalog are unchanged. The default cache
reuses the rendered result; `CachePolicy::None` formats again. Remove the binding
before taking over its text manually. Native `bypass_change_detection` writes
remain untracked; equal translated output preserves the target's change ticks.

## Prefer Msg for deferred values

`Msg<Scope>` is the primary formatter type. `Message<Scope>` remains a deprecated
alias, so existing constructors and values can be migrated incrementally:

```rust,ignore
use bevy_fluent_typed::{Localized, Msg};

let message = Msg::new(|catalog: &texts::ui::Greeting| catalog.msg_hello());
commands.spawn((Text::default(), Localized::<Text, _>::from(message)));
```

Creating or cloning a Msg is passive. Automatic updates begin only after an
associated `Localized` binding is inserted. A direct catalog accessor or
`message.render(&catalog)` returns a caller-owned String; passing that String to
an ordinary component does not establish automatic ownership.

## Bind custom components through the same contract

Implement `LocalizationTarget` directly on the application component:

```rust,ignore
use bevy::prelude::{Component, EntityWorldMut};
use bevy_fluent_typed::{LocalizationTarget, LocalizationUpdate};

#[derive(Component, Default)]
struct Caption {
    text: String,
}

impl LocalizationTarget for Caption {
    type Data = &'static mut Self;
    type Filter = ();

    fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
        let Some(target) = entity.get_mut::<Self>() else {
            return;
        };

        update.set_text(target.map_unchanged(|target| &mut target.text));
    }
}

```

Use `Localized<Caption, Scope>` beside the Caption. No component registration
plugin is required. `Data` and `Filter` describe one native query; default `update`
requests the sequential `apply` callback. Override only the callbacks needed by
the target. Applications construct the target and any neighboring components;
there is no `initialize` hook or automatic target creation. See the
[binding contract and examples](../bindings.md).

`Localized<AccessibleLabel, Scope>` is a separate binding on Bevy 0.19/0.20.
A Text binding does not create an AccessibleLabel. The label adapter uses Bevy's
ordinary insertion hooks; localization does not separately own or repair the
AccessibilityNode. Custom targets use the same public contract.

## Choose whether formatted output is retained

`CachePolicy::Rendered` is the default and retains formatted output per binding.
Set `.with_cache_policy(CachePolicy::None)` on the provider's LocalizationPlugin
to keep tracking metadata without that additional String. Target components
still own their strings. Native targets use change tracking to skip Idle;
custom callbacks format lazily when they request `update.text()` or `set_text`.
Use `is_changed()` and `component_changed()` to skip reads where appropriate,
including in deferred `apply`. See the [cache policy tradeoffs](../bindings.md#choose-a-providers-cache-policy).

Large eligible updates use Bevy's native query batching. The runtime does not
calibrate during startup or learn callback costs. A lower wall time can consume
more aggregate CPU; profile the application's frame budget before tuning.
Automatic Msg formatters and direct query callbacks may now execute concurrently
on compute workers. Keep them independent of main-thread identity and entity
iteration order; structural `apply` callbacks remain sequential. See the
[performance reference](../performance.md) for skipped work and measured tradeoffs.

## Verify the migration

Run `cargo check` and the application's tests after changing imports and target
types. Exercise a locale switch, changed message arguments, ordinary target edits
and target removal/reinsertion. Check visible text and accessibility bindings
independently. The [runnable binding example](../../examples/bindings) covers the
shared target API; its README states the scenarios and verification limits.
