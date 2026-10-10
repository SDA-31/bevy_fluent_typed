# Loading guide

Start with the [complete quickstart](README.md#setup), including its
dependencies, build script and first FTL file. The snippets below use those
generated types and Bevy 0.20.

## Contents

- [Choose automatic or manual loading](#choose-what-stays-loaded)
- [Automatic module lifetime](#automatic-module-lifetime)
- [Manual Lazy loading](#manual-lazy-loading)
- [Custom byte sources](#custom-byte-sources)
- [Without code generation](#without-code-generation)
- [Headless application](#headless-application)
- [Load and release a screen](#load-and-release-a-screen)
- [Read resources](#read-resources-and-handle-readiness)
- [Navigate from a parent](#navigate-from-the-root-or-a-parent)
- [Bind text](#bind-text-without-keeping-an-old-translation)
- [Change language and reload](#change-language-and-reload-files)
- [Scheduling and guarantees](#scheduling-and-reload-guarantees)
- [Migrate from 0.1.3](#migration-from-registry-013)
- [Troubleshooting](README.md#troubleshooting)

## Choose what stays loaded

A **module** is one FTL file. A **scope** is a generated leaf, a folder containing
leaves, or the whole `Translations` tree. Loading a scope requests all its leaves.

| Need | Plugin/controller mode | Requests |
| --- | --- | --- |
| Load only modules used by text or resource systems | `Auto` (default) | Inserted bindings and localized systems own their scopes |
| Keep the whole selected language ready, including direct world polling | `Full` | All modules automatically |
| Manage loading explicitly, including hybrid lifetimes | `Lazy` | Explicit `load` and `unload` |

Use `LocalizationPlugin::<texts::Translations>::new(texts::manifest())` for the
usual automatic path. With no consumers, no FTL modules are read or parsed.
A scope requests complete FTL leaves, not individual messages. Creating an
unattached `Message` or binding, inspecting the world, or navigating a controller
view is passive. `Auto` and `Full` have no `load` or `unload` methods.

## Automatic module lifetime

Insert a binding and let the plugin manage its scope:

```rust,ignore
commands.spawn((
    Text::default(),
    LocalizedText::<texts::presentation::Hud>::new(|hud| hud.msg_title()),
));
```

In `Auto`, every inserted `LocalizedText<Scope>` requests and retains its scope.
Removing the last binding releases that demand. Other bindings and localized
systems using overlapping scopes keep their required leaves loaded. No explicit
screen-entry or screen-exit loading calls are needed for bound text.

Register required resource systems with `add_localized_systems` or `localized`.
They acquire scope ownership when Bevy initializes the system, then wait until
its direct required `Res<Scope>` parameters are ready. Recurring systems retain
ownership while their system state exists, including when `run_if` is false;
a condition controls execution, not module lifetime. Remove the owning schedule
or use entity-bound text for lifetime that follows a screen's entities.

`add_localized_startup_systems` releases its temporary demand after the function
body returns, including returned errors. Bindings inserted through deferred
commands take over ownership before unneeded modules are released. A one-time
read does not keep the catalog loaded afterward.

Plain `app.add_systems(Update, function_with_res)` does not infer loading demand.
Use the existing localization helper for automatic required-resource consumers.
`Option<Res<Scope>>`, standalone messages, and `World::get_resource` inspection
neither request nor retain a module. For an application that polls resources
directly, select `LocalizationPlugin::<texts::Translations, Full>::new(manifest)`
and use the matching `Localization<texts::Translations, Full>` controller type.

After the last automatic consumer disappears, the plugin releases parsed
snapshots and strong asset handles at synchronization. Bevy may retire assets
on later updates. Application-owned clones may keep parsed data alive; embedded
static bytes and buffers passed to `from_bytes` have their own source lifetime.

The following examples add a pause screen and Spanish. Keep the quickstart's
manifest and English HUD; add these files:

`assets/localizations/translations/en/screens/pause.ftl`:

```ftl
title = Paused
```

`assets/localizations/translations/es/presentation/hud.ftl`:

```ftl
title = Listo
```

`assets/localizations/translations/es/screens/pause.ftl`:

```ftl
title = En pausa
```

All languages have the same module/message contract. Rebuild to generate the
new `texts::screens::Pause` type and `texts::Locale::Es` variant.

## Manual Lazy loading

Use this advanced recipe only when the application owns explicit requests.
Keep the quickstart's imports, `translations!` declaration and `show_title`.
Unlike Auto, Lazy does not let bindings or localized systems request modules.
Add the Lazy controller types:

```rust,ignore
use bevy_fluent_typed::{Lazy, Localization};

type AppLocalization = Localization<texts::Translations, Lazy>;
```

Replace `main` with this version:

```rust,ignore
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(bevy::asset::AssetPlugin {
            file_path: ".".into(),
            ..default()
        }))
        .add_plugins(LocalizationPlugin::<texts::Translations, Lazy>::new(texts::manifest()))
        .add_systems(Startup, request_hud)
        .add_localized_startup_systems(show_title)
        .run();
}
```

Add the explicit HUD request:

```rust,ignore
fn request_hud(mut localization: ResMut<AppLocalization>) {
    localization.load::<texts::presentation::Hud>();
}
```

This requests the HUD and prints `Ready` once it arrives. The pause module is
not requested, read or parsed. The request stays active until you unload it.
`LocalizationPlugin::<texts::Translations>::new_lazy(texts::manifest())` returns
the same plugin type. All three modes use the same `Res<texts::presentation::Hud>`.

With a file-backed manifest, `load::<Scope>()` starts the requested Bevy asset
loads at synchronization. The AssetLoader reads and parses each module during
that load, then the plugin publishes its typed resource. All modes use
Bevy's asynchronous loading; Lazy lets the application choose which scopes to
request and retain. No additional parsing is deferred until accessor use.

Use the same 0.3.0 facade in both normal and build dependencies for this recipe.
Then embed only the HUD by declaring its source next to `translations!`:

```rust,ignore
texts::embed_manifest! {
    const HUD = presentation::Hud;
}
```

Pass `HUD` in place of `texts::manifest()` and keep the request.
Embedding selects raw bytes in the binary; Lazy requests select what gets parsed.
Use `const ALL = Translations;` and pass `ALL` to include the pause screen too.
The existing expression form `texts::embed_manifest!()` also includes the whole tree.

### Export an embedded source from a private module

The source constant has type `LocalizationManifest`. Export it alongside ordinary
catalog aliases; the generated module can stay private:

```rust,ignore
mod localization {
    bevy_fluent_typed::translations!(mod texts);

    pub use texts::presentation::Hud as Interface;
    pub use texts::Translations;

    texts::embed_manifest! {
        pub const HUD = presentation::Hud;
    }
}
```

Consumers use the exported types and constant directly:

```rust,ignore
use localization::{HUD, Interface, Translations};

// During plugin setup:
app.add_plugins(LocalizationPlugin::<Translations>::new(HUD));
```

Insert `LocalizedText<Interface>` or register a direct `Res<Interface>` consumer
with `add_localized_systems`; Auto requests its scope without manual calls.
`Interface` names the catalog resource; `HUD` describes its embedded source.
The macro's selectors always use the original relative schema names. Multiple
constants, visibility and per-declaration `#[cfg(...)]` attributes are supported.
A constant includes bytes but does not parse a Fluent catalog. Source bytes stay
static after their last consumer disappears; parsed catalogs follow the usual resource lifetime.

## Load and release a screen

In the same Lazy application, call these systems when the pause screen opens and
closes. For example, register them in your state's `OnEnter` and `OnExit`
schedules; do not register both unconditionally in `Update`:

```rust,ignore
fn open_pause(mut localization: ResMut<AppLocalization>) {
    localization.load::<texts::screens::Pause>();
}

fn close_pause(mut localization: ResMut<AppLocalization>) {
    localization.unload::<texts::screens::Pause>();
}
```

Keep the HUD's request while the game uses it. This gives hybrid loading. Release
that request with `unload::<texts::presentation::Hud>()` when its owner exits too.

Requests are idempotent **per scope type**, not reference-counted calls. Two
`load::<Hud>()` calls followed by one `unload::<Hud>()` release that request. If
several application systems share ownership, coordinate that ownership in your
application. Independent parent/child requests do overlap:

```rust,ignore
localization.load::<texts::Presentation>();
localization.load::<texts::presentation::Hud>();
localization.unload::<texts::Presentation>(); // The explicit Hud request remains.
localization.unload::<texts::presentation::Hud>(); // Now it can be released.
```

`load::<texts::Translations>()` requests the whole tree. It removes the memory
benefit of partial loading while that request remains active. Subdivide very
large translations into useful FTL files: a leaf is parsed and retained as a
whole, not one message at a time.

After the last request is released, the plugin drops its scope snapshots and
strong asset handles at synchronization. Bevy may retire assets over later
updates. Application-owned clones can keep data alive, and explicitly embedded
static bytes always remain in the executable. There is no language cache or
implicit fallback.

## Read resources and handle readiness

Use native `Res<Scope>` when a system needs ready translations on every run:

```rust,ignore
use bevy_fluent_typed::LocalizationAppExt;

fn update_hud(hud: Res<texts::presentation::Hud>) {
    println!("{}", hud.msg_title());
}

app.add_localized_systems(Update, update_hud);
```

The library infers the required catalog types from direct `Res<Scope>` parameters.
Before loading, after unloading and during a locale transition, the system waits
without blocking the frame. It resumes when its catalogs are ready. A complete
parent waits for all its children; a HUD leaf does not wait for the pause module.
In Auto, the helper requests and retains the required scopes from system
initialization until the system state is dropped. This applies even when an
additional `run_if` is false. Explicit Lazy still needs `load`/`unload`; Full
keeps all modules of the selected language loaded automatically.

For systems that must also run while a catalog is absent, such as UI cleanup,
Bevy's `Option<Res<Scope>>` remains available. That optional parameter neither
waits for the catalog nor requests or retains it.

Functions and tuples are supported; tuple members wait independently. To add
normal Bevy scheduling configuration, wrap functions before configuring them:

```rust,ignore
use bevy_fluent_typed::localized;

app.add_systems(Update, localized(update_hud).run_if(screen_is_open));
```

For initialization that must happen once after loading:

```rust,ignore
app.add_localized_startup_systems(setup_hud);

fn setup_hud(mut commands: Commands, hud: Res<texts::presentation::Hud>) {
    commands.spawn(Text::new(hud.msg_title()));
}
```

This helper runs in `Update`, remembers actual invocation and applies normal
Bevy deferred commands. In Auto it releases its temporary scope ownership after
the body returns; inserted bindings retain their own demand. It does not rerun
after unloading or language changes; use `LocalizedText` for text that must stay live. A function returning an error
still counts as invoked, and Bevy handles that error normally. These functions
have no ordering relationship with ordinary `Startup` systems beyond running
later. Tuple members are independent. This helper wraps each function, so
`.before(setup_hud)` / `.after(setup_hud)` do not target the deferred wrapper.
Combine dependent initialization steps in one function, or use recurring
`localized(...)` systems with application-owned initialization state and ordering.

Readiness inference supports direct native `Res<Scope>` parameters of functions
and closures. Custom derived `SystemParam`s, `ParamSet` and nested parameter tuples
are not inspected: expose each required catalog as a direct parameter. For
advanced forms, manage demand with explicit Full/Lazy and provide readiness
conditions separately. Install the localization plugin before the schedule is
initialized so its catalog types are registered. Apply configuration after
`localized`; already configured systems have erased their parameter types.
Missing ordinary resources retain Bevy's normal validation behavior.

The ordinary `add_systems` API is unchanged. The new helper requires a recurring
schedule, such as `Update`, to retry waiting systems. Ordinary `Startup`, `OnEnter`
and other one-shot schedules do not retry; use deferred initialization above when
needed. `add_localized_systems` rejects the three built-in startup schedules to
avoid silently losing an initialization step.

`Localization::new(locale)` and `Default` create a controller, not a ready catalog.
Use `status::<Scope>()` to distinguish `Unloaded`, `Loading`, `Ready` and
`Failed(error)`. A separate observer can report load errors while required systems
wait. An invalid same-language reload preserves the last good value, so its
consumers can run while the latest attempt has status `Failed`. Send
`ReloadCatalogs` to retry requested leaves; in explicit Lazy, repeating
`load::<Scope>()` also retries failed leaves.

## Navigate from the root or a parent

The controller's views work with a partially loaded tree:

```rust,ignore
fn inspect(localization: Res<AppLocalization>) {
    let presentation = localization.modules().presentation();

    if let Ok(hud) = presentation.hud() {
        println!("{}", hud.msg_title());
    }
}
```

Group methods return another view. Leaf methods return `Result<&Leaf,
ModuleError>` with the locale, logical module path and loading status. Views do
not load anything. A leaf borrow belongs to the controller's store, so a chained
call such as `localization.modules().presentation().hud()` is also valid.

With a complete scope resource, ordinary accessors return direct references:

```rust,ignore
fn inspect_complete(presentation: Res<texts::Presentation>) {
    let hud = presentation.hud();
    println!("{}", hud.msg_title());
}
```

Register that system with `app.add_localized_systems(Update, inspect_complete)`.
`localization.catalog()` returns `Option<&texts::Translations>`; it is normally
`None` in an application that intentionally leaves some modules unloaded.

## Bind text without keeping an old translation

A `LocalizedText<Scope>` component stores a closure. The plugin renders it using
the current resource and updates the existing `Text` or `Text2d` component when
the language, module or binding changes:

```rust,ignore
use bevy_fluent_typed::LocalizedText;

fn spawn_title(mut commands: Commands) {
    commands.spawn((
        Text::default(),
        LocalizedText::<texts::presentation::Hud>::new(|hud| hud.msg_title()),
    ));
}
```

Register `spawn_title` in `Startup`; it is safe to create a binding before its
module loads. In Auto, insertion requests the HUD and removal releases that
binding's demand. Your application owns the usual Bevy UI/camera/font setup.
Missing or unloaded resources clear bound text, and it
refreshes when the resource becomes available. Bind to the smallest scope the
message uses: a `LocalizedText<Translations>` would wait for the entire tree.

Bindings can be cloned and used in native Bevy 0.19/0.20 scenes; see the
[BSN recipe](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/bsn.md) for the syntax of each backend.

For stored notices, use `Message<Scope>` and render only when its scope is ready.
A stored message does not establish demand until inserted as a `LocalizedText`
binding; render it in a localized required-resource system when needed:

```rust,ignore
use bevy_fluent_typed::Message;

let message = Message::new(|hud: &texts::presentation::Hud| hud.msg_title());
// Or: Message::<texts::presentation::Hud>::new(|hud| hud.msg_title())

if let Ok(hud) = localization.modules().presentation().hud() {
    let text = message.render(hud);
}
```

Capture owned message arguments, not a translated string or an old module
snapshot. Replace the binding when captured values change. Editable text drafts
should use separate components: localization replaces bound text. Number
formatting stays application-owned; see the ICU4X example.

## Change language and reload files

For the quickstart's locales, use the matching default controller alias:

```rust,ignore
use bevy_fluent_typed::Localization;
type AppLocalization = Localization<texts::Translations>;
```

Then select Spanish:

```rust,ignore
fn select_spanish(mut localization: ResMut<AppLocalization>) {
    localization.set_locale(texts::Locale::Es);
}
```

Call it from your language-selection action. Existing scope requests stay active;
previous-language data is cleared and the same requested modules load in Spanish.
Bindings update as their resources become ready. No old-language fallback is used.
For a different initial language, insert `AppLocalization::new(texts::Locale::Es)`
before installing the plugin. Otherwise the manifest's default language is used.

To keep the active language usable while acquiring a target, prepare it first:

```rust,ignore
use bevy_fluent_typed::PreparationStatus;

localization.prepare_locale(texts::Locale::Es);
// Poll in a later update; required Res<Scope> consumers keep using the active locale.
if localization.preparation_status() == PreparationStatus::Ready {
    localization.commit_locale()?;
}
```

`prepared_locale()` reports the target, including during acquisition. Preparation
loads exactly the current requests in either Full or Lazy mode, using the plugin's
existing source and checked parsers. Target leaves remain private: native resources,
controller views and bound text continue using the active locale. `commit_locale()`
returns `CommitLocaleError::NotPrepared`, `Pending` or `Failed(ModuleError)` until
all requested target leaves passed their latest attempt; a retained earlier good
snapshot cannot satisfy a failed retry. A successful call queues an atomic locale
swap for the next PreUpdate `LocalizationSystems::Publish` boundary. `locale()`
changes there, together with every available native scope; bindings refresh in
PostUpdate. No target loading notifications appear in `CatalogUpdate` before
commit; committed requested leaves emit `Loaded`.

Repeating `prepare_locale` for the same target keeps its successful leaves and
retries failures. A different target replaces it. Lazy `load`/`unload` requests
also update target demand; changing demand or requesting a retry revokes a queued
commit. Poll readiness and commit again. Publication rechecks readiness and revokes
the commit if a newly observed target source failure makes it unavailable.
`cancel_preparation()` drops target snapshots and tasks/handles. External I/O may
finish later, but canceled attempts cannot satisfy a replacement preparation.
`set_locale()` cancels preparation, including when selecting the active locale.
Preparing the active locale is immediately ready, preserves snapshots and performs
no extra I/O. An empty Lazy request set is also ready without loading any modules.

Preparation retains active and target parsed leaves until cancellation or commit;
there is no retained locale cache. Application-owned clones and the source's own
buffers retain their usual lifetime. File sources use Bevy's normal AssetReader
and AssetLoader with a private typed preparation asset and per-attempt identities.
Repreparing or retrying the same file waits asynchronously for its previous
preparation handle to retire. Before commit, target files also wait for obsolete
normal catalog handles to retire so the handoff cannot reconnect a canceled
reader. An uncancelable reader opening the source or another owner retaining
such a handle may delay readiness. Cancellation returns immediately and active
catalog consumers keep running throughout. Source metadata must allow Bevy to select the requested
asset type; explicit `.meta` files selecting a different loader cause preparation
to fail. Remove that loader override or configure Bevy's metadata policy for your
source. Automatic watching and `ReloadCatalogs` continue to work after commit.
This does not snapshot a changing archive or order overlapping external reloads;
keep the source coherent while preparing, as described below.

To retry all requested FTL files on Bevy 0.17–0.20:

```rust,ignore
use bevy_fluent_typed::ReloadCatalogs;

fn reload(mut requests: MessageWriter<ReloadCatalogs<texts::Translations>>) {
    requests.write(ReloadCatalogs::default());
}
```

Register this on your reload action, not every frame. Bevy 0.16 uses `EventWriter`
and `send`. The request works without filesystem watching and can recover from
initially missing files. It does not reread TOML: the manifest remains the
contract you passed to the plugin.

To watch files automatically, enable the runtime's `watch` feature on the normal
dependency and set `AssetPlugin::watch_for_changes_override` to `Some(true)`.
The selected asset source must support watching. Compatible prose edits can load
without recompiling; changing the schema requires rebuilding the application.

For load diagnostics, use `CatalogUpdateReader<texts::Translations>` and iterate
`.read()`. `CatalogUpdate::Loaded { locale, path }` identifies an accepted module;
`Rejected { locale, path, error }` reports a failed attempt. In `PreUpdate`, order
the observer after `LocalizationSystems::Publish`. Error presentation belongs to
the application.

## Scheduling and reload guarantees

Publication runs in PreUpdate's `LocalizationSystems::Publish`, then again in
PostUpdate before `LocalizationSystems::Refresh`, where text bindings refresh.
A language change in Update changes the controller immediately; scope resources
follow at publication. Consumers that must observe the synchronized result in
that frame should run in PostUpdate after `LocalizationSystems::Refresh`.

Checked leaves publish independently. A bad same-language reload keeps that
leaf's last good value; valid siblings can still change. Ordinary same-language reloads have no multi-file
transaction; explicit locale preparation commits its requested target leaves together. Successful reloads publish fresh snapshots even if text is identical;
idle frames and unchanged siblings preserve resource identity and change ticks.

The plugin serializes its own reload requests per module and coalesces pending
retries. Watcher reloads and direct `AssetServer::reload` calls bypass that queue.
Bevy exposes no request generation before opening the reader or in a failure
event, so overlapping external reloads cannot always be ordered by request time.
If strict order matters, disable automatic watching and send `ReloadCatalogs`;
a custom reader must supply coherent data as well.

## Custom byte sources

For on-demand reading and unloading, use [Auto](#automatic-module-lifetime)
with Bevy's asset system, or [manual Lazy loading](#manual-lazy-loading). [Custom asset sources](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/asset-sources.md) let the
application supply transport, decompression and caching through an `AssetReader`.

`from_bytes` is for data the application has already obtained and intends to keep
in memory. **The plugin retains all supplied buffers for later loads and language
changes.** In Auto, only parsing and resource creation wait for consumers;
removing consumers releases unneeded parsed resources, but retains input buffers.
Explicit Lazy follows manual requests with the same buffer lifetime.

Keep `codegen`, `build.rs` and `translations!`. Generated accessors work with
these buffers too; only the plugin constructor changes. This path needs
`MinimalPlugins` or `DefaultPlugins`, without `AssetPlugin`.

For the guide's two English modules, read both files and keep the default Auto:

```rust,ignore
let hud = std::fs::read("assets/localizations/translations/en/presentation/hud.ftl")?;
let pause = std::fs::read("assets/localizations/translations/en/screens/pause.ftl")?;
let plugin = LocalizationPlugin::<texts::Translations>::from_bytes([
    (texts::Locale::En, texts::presentation::Hud::PATH, hud),
    (texts::Locale::En, texts::screens::Pause::PATH, pause),
])?;
app.add_plugins(plugin);
```

The usual inserted bindings and localized required `Res` systems request only
the modules they need; no explicit `load` call is needed. Supply Spanish buffers
too if the application will switch to Spanish.

`from_bytes` accepts `(locale, leaf path, bytes)` tuples, checks their keys
immediately, and validates FTL when a module is parsed. A missing buffer becomes
`ModuleStatus::Failed` when requested. The provider's default locale is selected
initially, unless a `Localization<C>` resource was inserted before the plugin.

For a runnable complete example, see [generated bytes](https://github.com/SDA-31/bevy_fluent_typed/blob/main/examples/codegen/src/bin/bytes.rs):

```sh
cargo run --manifest-path examples/codegen/Cargo.toml --bin bytes
```

For applications that already own asynchronous acquisition outside Bevy's asset
system, `from_loader` accepts a callback receiving `(locale, leaf path)` and
returning a future with owned readable bytes or an error. It runs the checked
parser and drops the temporary input afterward. Application captures and caches
can still retain data. The callback and parser run on Bevy's I/O task pool;
futures must be nonblocking and any required executor integration is owned by
the application. Reload notifications and explicit Lazy retries use the same
callback. This path has no AssetServer file watching; send
`ReloadCatalogs` after source changes. See `LocalizationPlugin::from_loader`
for cancellation, scheduling and reload guarantees.

## Without code generation

This is an advanced integration for applications supplying their own provider.
Most applications should keep the generated API above, including when they
provide their own bytes.

For Bevy 0.20, the minimal dependency is:

```toml
[dependencies]
bevy_fluent_typed = { version = "0.3.0", default-features = false, features = ["bevy-0-20"] }
```

There is no build-dependency, `build.rs`, `translations!`, TOML manifest or
`fluent_typed_codegen` dependency in this configuration. Implement `FluentCatalog`
and `FluentScope`, register checked leaf parsers through `Module::new`, and pass
bytes with `from_bytes` or `from_loader`. The application owns the locale/module
schema, accessor API and validation. Bytes alone cannot create typed Rust methods.
The plugin still manages automatic consumers or explicit Full/Lazy requests,
typed resources, readiness, bindings, language changes, retries and unloading.

Follow the [complete handwritten example](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/no_codegen) for these
implementations. Add `manifest` if that provider needs the existing manifest/file
constructors. `manifest` is enabled by default and by `codegen`; it uses the
generator package's small runtime manifest API, without enabling generation.
See the [feature table](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/build.md#features) for host/target separation.

## Headless application

For a console-only example, use `MinimalPlugins`, `AssetPlugin` and a repeating
runner. Build `src/main.rs` from these three blocks; the application prints
`Ready` and exits, or returns an error if the HUD cannot load.

The imports and generated module:

```rust,ignore
use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    prelude::*,
};
use bevy_fluent_typed::{
    Localization, LocalizationAppExt, LocalizationPlugin, ModuleStatus,
};
use std::{path::Path, time::Duration};

bevy_fluent_typed::translations!(mod texts);
```

The console event loop and asset root:

```rust,ignore
fn main() -> AppExit {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR"));

    App::new()
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))),
            AssetPlugin {
                file_path: assets.to_string_lossy().into_owned(),
                ..default()
            },
            LocalizationPlugin::<texts::Translations>::new(texts::manifest()),
        ))
        .add_localized_startup_systems(show_title)
        .add_systems(Update, report_failure)
        .run()
}
```

The output and failure handling:

```rust,ignore
fn show_title(
    hud: Res<texts::presentation::Hud>,
    mut exit: MessageWriter<AppExit>,
) {
    println!("{}", hud.msg_title());
    exit.write(AppExit::Success);
}

fn report_failure(
    localization: Res<Localization<texts::Translations>>,
    mut exit: MessageWriter<AppExit>,
) {
    if let ModuleStatus::Failed(error) = localization.status::<texts::presentation::Hud>() {
        eprintln!("{error}");
        exit.write(AppExit::error());
    }
}
```

The absolute asset root here makes this standalone console executable runnable
from another working directory. A normal Bevy application uses its existing
asset configuration and event loop, as in the quickstart.

## Migration from registry 0.1.3

Follow the [migration guide to 0.3.0](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/migration-0.2.md) for dependency updates, before/after
initialization, readiness handling, optional Lazy adoption and handwritten providers.
The standard upgrade uses Auto with inserted bindings and localized systems;
choose explicit Full to preserve eager loading or direct world polling.

## Compile-time mode boundaries

`Auto`, `Full` and `Lazy` implement the sealed `LoadingMode` trait. Explicit
requests exist only on Lazy. The compiler rejects requests on Full and scopes
belonging to another root:

```compile_fail,E0599
use bevy_fluent_typed::{FluentCatalog, Full, Localization};
fn request<C: FluentCatalog>(state: &mut Localization<C, Full>) {
    state.load::<C>();
}
```

```compile_fail,E0599
use bevy_fluent_typed::{FluentCatalog, Full, Localization};
fn release<C: FluentCatalog>(state: &mut Localization<C, Full>) {
    state.unload::<C>();
}
```

```compile_fail
use bevy_fluent_typed::{FluentCatalog, FluentScope, Lazy, Localization};
fn wrong_root<C: FluentCatalog, S: FluentScope>(state: &mut Localization<C, Lazy>) {
    state.load::<S>(); // S::Catalog must be C.
}
```

```compile_fail
use bevy_fluent_typed::LoadingMode;
struct Other;
impl LoadingMode for Other { const FULL: bool = false; }
```

## More recipes

- [Custom sources](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/asset-sources.md): supply files through your own reader.
- [Decimal and percentage arguments](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/formatting.md): reuse ICU4X formatters.
- [Build API](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/build.md): explicit generation and feature selection.
