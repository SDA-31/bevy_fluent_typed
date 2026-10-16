# Migrate from bevy_fluent_typed 0.3.1 to 0.4.0

This migration replaces implicit text targets with explicit bindings. Loading
modes, generated catalogs, startup strategies, supported Bevy families and the
Rust 1.97.1 minimum remain unchanged. The generator stays at 0.2.2.
The [changelog](../CHANGELOG.md#040) records this version's release status.

## Update both dependency entries

```toml
[dependencies]
bevy = "0.20"
bevy_fluent_typed = { version = "0.4.0", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.4.0", default-features = false, features = ["build"] }
```

Select another Bevy backend on the normal dependency with defaults disabled,
as in the [compatibility recipe](../README.md#supported-engines). Do not forward
backend features to the build dependency. The existing explicit `build.rs`
generation call and `translations!` declaration do not change.

The optional derive implementation is a separate package, enabled through the
runtime's `derive` feature and included by `codegen`. Applications do not need a
second dependency or localization plugin to use it.

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

Built-in targets with Default are created once if absent when the binding is
attached. Existing targets and their settings are preserved. Removing a target
later leaves it absent even after message or locale changes. Reinsert the target
to resume, or remove and add the binding for a new initialization.

Removing only the binding stops automatic writes and releases its scope demand;
the target keeps its last value. Other bindings and resource consumers can
still retain the scope. A target without Default must be supplied by the
application unless its manual adapter implements an initialization policy.

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

For a mutable Default component with one localized String field:

```rust,ignore
use bevy::prelude::Component;
use bevy_fluent_typed::LocalizationTarget;

#[derive(Component, Default, LocalizationTarget)]
struct Caption {
    #[localized]
    text: String,
}
```

Use `Localized<Caption, Scope>` beside the Caption. No component registration
plugin is required. Non-Default components, immutable components and compound
targets implement `LocalizationTarget` manually. `Data` and `Filter` describe
one native query; default `update` requests the sequential `apply` callback.
Override only the callbacks needed by the target. See the
[binding contract and examples](bindings.md).

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
including in deferred `apply`. See the [cache policy tradeoffs](bindings.md#choose-a-providers-cache-policy).

Large eligible updates use Bevy's native query batching. The runtime does not
calibrate during startup or learn callback costs. A lower wall time can consume
more aggregate CPU; profile the application's frame budget before tuning.

## Verify the migration

Run `cargo check` and the application's tests after changing imports and target
types. Exercise a locale switch, changed message arguments, ordinary target edits
and target removal/reinsertion. Check visible text and accessibility bindings
independently. The [runnable binding example](../examples/bindings) covers the
shared target API; its README states the scenarios and verification limits.
