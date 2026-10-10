# Migrate from bevy_fluent_typed 0.2.2 to 0.3

Runtime 0.3.0 supports stable Bevy 0.20 and requires Rust 1.97.1 or newer.
The generator stays at 0.2.2. Constructor signatures, native resources and
embedded manifest declarations remain available, but the default loading mode
changes from eager Full to consumer-driven Lazy.

## Adopt automatic module loading

The unchanged common constructor now returns a Lazy plugin:

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

No consumers, manual pins or leases means no module reads or parsing. Plain `add_systems`, optional
resources, `World::get_resource` inspection and unattached messages do not
request modules. Keep explicit Full if your application directly polls the
world or needs a complete language loaded independently of consumers:

```rust,ignore
use bevy_fluent_typed::{Full, Localization, LocalizationPlugin};

app.add_plugins(LocalizationPlugin::<texts::Translations, Full>::new(texts::manifest()));
// Any controller parameters or preinserted controllers must use this mode too:
type AppLocalization = Localization<texts::Translations, Full>;
```

Lazy also accepts optional `load::<Scope>()`/`unload::<Scope>()` calls to preload
or retain a scope independently of consumers. These are idempotent manual pins:
one unload removes the pin after repeated loads, but cannot evict automatic
consumers, held leases or other overlapping pins. `hold::<Scope>()` creates an
independent owner in Lazy or Manual; its Drop releases only that owner.
For existing 0.2.2 applications, rename the explicit-request mode `Lazy` to
`Manual` and replace `new_lazy(manifest)` with `new_manual(manifest)`.
The new `Lazy` is consumer-driven and is the default; explicit `Manual` keeps
the previous manual `load`/`unload` behavior. Use the same mode on the plugin and
its controller; resource scopes such as `Res<Hud>` do not change. See
[module lifetime](../GUIDE.md#automatic-module-lifetime).

## Let language changes finish automatically

Keep the usual `localization.set_locale(target)` call. It now prepares all
currently desired target leaves, keeps the active language's resources and text
usable, and switches automatically after validation at PreUpdate Publish.
`locale()` reports the active language until that switch; `prepared_locale()`
and `preparation_status()` inspect the target. Automatic consumers, manual pins
and leases added or removed during acquisition update the target demand.

The latest target wins; selecting the active language cancels a pending target
and retries failed active leaves without reloading ready ones or adding pins.
Repeating a pending target does not start duplicate loads. A failed target emits
`Rejected` for that language and keeps the active resources; there is no per-frame
retry loop. Repeat `set_locale(target)`, retry a failed requested scope with
`load`, or send `ReloadCatalogs` after correcting the source.

Manual `prepare_locale`/`commit_locale` remains an advanced option for choosing
publication time. Calling `prepare_locale` takes over manual control even for the
same pending automatic target. `cancel_preparation` cancels either kind of switch.
Same-language file reloads still publish independently; coordinated language
replacement does not provide a snapshot of a changing external source.

## Use text spans

The same `LocalizedText<Scope>` binding now updates a `TextSpan` child of a UI
or world-text root. It has the same automatic scope ownership and locale-switch
behavior as `Text`/`Text2d`; it does not create a root text component. The
application keeps ownership of hierarchy and style. See the [span recipe](bsn.md#localize-a-text-span).

## Native loading progress

Add one root progress plugin for the native views your loading UI needs:

```rust,ignore
use bevy_fluent_typed::LocalizationProgressPlugin;

app.add_plugins(LocalizationProgressPlugin::<texts::Translations>::new());
```

The root recursively includes every registered scope, including the HUD view
used below. A group plugin is an alternative that observes only its subtree;
a leaf plugin selects only that leaf. Generated hierarchy metadata is automatic.
During App setup, either order relative to the base localization plugin works.
The owning provider supplies Lazy/Full/Manual; overlapping and repeated registration is
idempotent and shares one dispatcher per provider. Native `LocalizationProgress<Scope>` resources are
initialized before `Startup` once the base plugin is installed. No Cargo feature
is needed. Without progress plugins, their resources and recurring trackers are
absent; ordinary loading and status remain available. Progress never requests
translations or retains parsed catalogs.

Read `Res<LocalizationProgress<texts::presentation::Hud>>` in Update, or after
`LocalizationSystems::Progress` in PostUpdate. `resource_changed` gates observers
on visible changes. Root snapshots count current demand; group/leaf snapshots
count fixed unique schema paths, including unrequested leaves. Their active and
target counts are scoped, including automatic target preparation;
`preparation_status()` describes provider-wide
commit readiness. A ready scope cannot establish global readiness; an unused
scope may remain unloaded while the provider is ready. Retries and asset-handle
handoff can also delay commit. `set_locale` publishes automatically when ready;
only explicit `prepare_locale` requires the application's `commit_locale` call. See [loading progress](../GUIDE.md#observe-loading-progress).

Per-scope `progress::<Scope>()` queries remain passive and include unrequested
schema leaves. They do not enable recurring tracking. `LoadingProgress` contains
counters only. Optional `diagnostics` enables explicit `diagnostics::<Scope>()`
and `preparation_diagnostics::<Scope>()` queries for individual module paths,
errors and availability; it is disabled by default and does not activate native
tracking. The registered publisher never collects those detail vectors, even
when the feature is enabled. See [module details](../GUIDE.md#optional-module-details).

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
