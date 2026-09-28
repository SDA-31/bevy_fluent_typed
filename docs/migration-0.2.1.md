# Adopt resource waiting in 0.2.1

Use native required resources with registration helpers that infer catalog
readiness. Existing manual conditions and explicit Lazy requests keep working.
Update both normal and build dependencies on `bevy_fluent_typed` to `0.2.1`.
The runtime integrates the Bevy build adapter and uses generator 0.2.1; no generated
schema or manifest changes are required.

## Use the build-configured file manifest

The generated helper replaces manual parsing of the build metadata:

```rust,ignore
LocalizationPlugin::<texts::Translations>::new(texts::manifest())
```

It performs no file reads and includes no FTL payload. Bevy's asset root is
unchanged. Existing explicit `LocalizationManifest` and `embed_manifest!` calls
keep working when runtime storage differs from the build configuration.

## Wait for a required catalog

Before:

```rust,ignore
fn update_hud(hud: Res<texts::presentation::Hud>) {
    println!("{}", hud.msg_title());
}

app.add_systems(
    Update,
    update_hud.run_if(resource_exists::<texts::presentation::Hud>),
);
```

After:

```rust,ignore
use bevy_fluent_typed::LocalizationAppExt;

fn update_hud(hud: Res<texts::presentation::Hud>) {
    println!("{}", hud.msg_title());
}

app.add_localized_systems(Update, update_hud);
```

`Res` is Bevy's normal resource parameter. The registration helper infers the
required catalog and skips the system until it is ready. It does not block the
frame or change the behavior of ordinary `add_systems`. With several required
catalog parameters, all must be ready.

Skipping a system does not clear text or undo its previous effects. Use
`LocalizedText` for labels that should clear automatically while unloaded.
Load errors remain observable through `status` and `CatalogUpdate`; handle them
in a system that does not require the missing catalog. Same-language reload failures retain the last good catalog.

## Preserve scheduling configuration

```rust,ignore
use bevy_fluent_typed::localized;

app.add_systems(Update, localized(update_hud).run_if(screen_is_open));
app.add_systems(Update, localized((update_title, update_buttons)).chain());
```

Pass functions or tuples before applying configuration. Each tuple member waits
independently. Original function ordering references continue to work for
recurring systems, as do Bevy sets, conditions and deferred commands.

## Initialize once after loading

```rust,ignore
app.add_localized_startup_systems(setup_hud);

fn setup_hud(mut commands: Commands, hud: Res<texts::presentation::Hud>) {
    commands.spawn(Text::new(hud.msg_title()));
}
```

The helper runs in `Update`, after ordinary startup, and records completion only
when the function body returns. Each tuple member completes independently. It
does not rerun after a locale change; use `LocalizedText` for live bindings.
A returned error still counts as one invocation and follows normal Bevy error
handling. The helper wraps each function, so
`.before(setup_hud)` / `.after(setup_hud)` do not order against the deferred
wrapper. Combine dependent steps in one function, or use recurring `localized(...)`
with application-owned initialization state and normal scheduling configuration.

Ordinary `Startup`, `OnEnter` and other one-shot schedules do not retry a skipped
system. The recurring helper rejects the three built-in startup schedules; custom
one-shot schedules must also be avoided. To initialize each time a screen opens,
use your screen's recurring loading/ready state rather than this application-wide
one-time helper.

## Loading ownership and supported parameters

Full mode still requests the selected language automatically. Lazy mode still
needs `load::<Scope>()` and `unload::<Scope>()`; waiting systems do not create
requests or prevent unloading. This keeps module lifetime under application control.

Inference covers direct native `Res<Scope>` parameters of functions and closures.
Custom derived `SystemParam`s, `ParamSet` and nested parameter tuples are not
inspected. Expose required catalogs directly or retain explicit conditions for
these forms. Missing non-catalog resources and application errors keep their
normal Bevy behavior. No global error handler is replaced.

See the [loading guide](../GUIDE.md#read-resources-and-handle-readiness) for
module lifetime, loading modes and typed navigation.

## Byte sources and optional manifest support

Existing default and codegen setups keep their manifest constructors. The build
adapter is now inside `bevy_fluent_typed`; remove any direct bridge dependency.
For handwritten providers with defaults disabled, add `manifest` if you use
`LocalizationManifest` or `LocalizationPlugin::new`/`new_lazy`. A backend alone
supports `from_bytes` and `from_loader` without the generator package. See the
[feature table](build.md#features) and [advanced guide](../GUIDE.md#custom-byte-sources).
