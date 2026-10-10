# Migrate from bevy_fluent_typed 0.2.2 to 0.3

Runtime 0.3.0 supports stable Bevy 0.20 and requires Rust 1.97.1 or newer.
The generator stays at 0.2.2. Localization constructors, Full/Lazy requests,
native resources, readiness helpers and embedded manifest declarations are unchanged.

## Native loading progress

The plugin automatically inserts and updates `LocalizationProgress<C>`. Read it
with native `Res<LocalizationProgress<texts::Translations>>` from your loading UI;
no additional feature, resource initialization or tracking system is needed.
`resource_changed` gates observers on visible changes. `active()` reports current
demand and `preparation()` reports an explicit target. Use `preparation_status()`
for commit readiness; ready leaf counts alone do not account for retries or
asset-handle handoff. Publication still requires the application's `commit_locale`
call. See [loading progress](../GUIDE.md#observe-loading-progress).

Per-scope `progress::<Scope>()` queries remain passive and include unrequested
schema leaves. `LoadingProgress` contains counters only. Optional `diagnostics`
enables explicit `diagnostics::<Scope>()` and `preparation_diagnostics::<Scope>()`
queries for individual module paths, errors and availability; it is disabled by
default. The automatic publisher never collects those detail vectors, even when
the feature is enabled. See [module details](../GUIDE.md#optional-module-details).

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
