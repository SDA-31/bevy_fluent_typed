# Loading guide

Start with the [complete quickstart](README.md#setup), including its
dependencies, build script and first FTL file. The snippets below use those
generated types and Bevy 0.20.

## Contents

- [Choose automatic or manual loading](#choose-what-stays-loaded)
- [Automatic module lifetime](#automatic-module-lifetime)
- [Manual loading](#manual-loading)
- [Custom byte sources](#custom-byte-sources)
- [Without code generation](#without-code-generation)
- [Headless application](#headless-application)
- [Load and release a screen](#load-and-release-a-screen)
- [Read resources](#read-resources-and-handle-readiness)
- [Loading progress](#observe-loading-progress)
- [Optional module details](#optional-module-details)
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
| Load only modules used by text or resource systems | `Lazy` (default) | Inserted bindings and localized systems own their scopes |
| Keep the whole selected language ready, including direct world polling | `Full` | All modules automatically |
| Manage loading explicitly, including hybrid lifetimes | `Manual` | Explicit `load` and `unload` |

Use `LocalizationPlugin::<texts::Translations>::new(texts::manifest())` for the
usual automatic path. With no consumers, manual pins or leases, no FTL modules
are read or parsed.
A scope requests complete FTL leaves, not individual messages. Creating an
unattached `Message` or binding, inspecting the world, or navigating a controller
view is passive. Lazy also supports optional manual pins; Full has no
`load` or `unload` methods.

## Automatic module lifetime

Insert a binding and let the plugin manage its scope:

```rust,ignore
commands.spawn((
    Text::default(),
    LocalizedText::<texts::presentation::Hud>::new(|hud| hud.msg_title()),
));
```

In `Lazy`, every inserted `LocalizedText<Scope>` requests and retains its scope.
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
neither request nor retain a module. For an application that polls the complete language
directly, select `LocalizationPlugin::<texts::Translations, Full>::new(manifest)`
and use the matching `Localization<texts::Translations, Full>` controller type.

After the last consumer, manual pin and lease disappear, the plugin releases parsed
snapshots and strong asset handles at synchronization. Bevy may retire assets
on later updates. Application-owned clones may keep parsed data alive; embedded
static bytes and buffers passed to `from_bytes` have their own source lifetime.

### Optional manual retention

Most text and resource consumers need no additional calls. To preload a scope or
keep it available independently of consumers, use the same Lazy controller:

```rust,ignore
// In a system with ResMut<Localization<texts::Translations>>:
localization.load::<texts::presentation::Hud>();
// When that extra retention is no longer needed:
localization.unload::<texts::presentation::Hud>();
```

`load` creates an idempotent manual pin per scope type. Two loads followed by one
unload remove that pin; automatic consumers and other overlapping pins still
retain their leaves. `unload` cannot evict a scope used by a binding or localized
system or held lease. Pins persist across language changes. `hold::<Scope>()`
adds independent owners in Lazy or Manual when needed; see the [manual ownership
recipe](#load-and-release-a-screen). Explicit Manual uses these manual APIs without
automatic consumers; Full always requests the complete language.

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

## Manual loading

Use this advanced recipe only when the application owns explicit requests.
Keep the quickstart's imports, `translations!` declaration and `show_title`.
Unlike Lazy, Manual does not let bindings or localized systems request modules.
Add the Manual controller types:

```rust,ignore
use bevy_fluent_typed::{Manual, Localization};

type AppLocalization = Localization<texts::Translations, Manual>;
```

Replace `main` with this version:

```rust,ignore
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(bevy::asset::AssetPlugin {
            file_path: ".".into(),
            ..default()
        }))
        .add_plugins(LocalizationPlugin::<texts::Translations, Manual>::new(texts::manifest()))
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
`LocalizationPlugin::<texts::Translations>::new_manual(texts::manifest())` returns
the same plugin type. All three modes use the same `Res<texts::presentation::Hud>`.

With a file-backed manifest, `load::<Scope>()` starts the requested Bevy asset
loads at synchronization. The AssetLoader reads and parses each module during
that load, then the plugin publishes its typed resource. All modes use
Bevy's asynchronous loading; Manual lets the application choose which scopes to
request and retain. No additional parsing is deferred until accessor use.

Use the same 0.3.0 facade in both normal and build dependencies for this recipe.
Then embed only the HUD by declaring its source next to `translations!`:

```rust,ignore
texts::embed_manifest! {
    const HUD = presentation::Hud;
}
```

Pass `HUD` in place of `texts::manifest()` and keep the request.
Embedding selects raw bytes in the binary; Manual requests select what gets parsed.
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
with `add_localized_systems`; Lazy requests its scope without manual calls.
`Interface` names the catalog resource; `HUD` describes its embedded source.
The macro's selectors always use the original relative schema names. Multiple
constants, visibility and per-declaration `#[cfg(...)]` attributes are supported.
A constant includes bytes but does not parse a Fluent catalog. Source bytes stay
static after their last consumer disappears; parsed catalogs follow the usual resource lifetime.

## Load and release a screen

In the same Manual application, call these systems when the pause screen opens and
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
`load::<Hud>()` calls followed by one `unload::<Hud>()` release that request.
Independent parent/child requests do overlap:

```rust,ignore
localization.load::<texts::Presentation>();
localization.load::<texts::presentation::Hud>();
localization.unload::<texts::Presentation>(); // The explicit Hud request remains.
localization.unload::<texts::presentation::Hud>(); // Now it can be released.
```

For independent owners or preloading, Lazy and Manual both support a
`ModuleLease<Scope>` returned by `hold`. This Manual example keeps screen entry/exit
under application control:

```rust,ignore
use bevy_fluent_typed::ModuleLease;

fn open_pause(mut commands: Commands, mut localization: ResMut<AppLocalization>) {
    let lease: ModuleLease<texts::screens::Pause> =
        localization.hold::<texts::screens::Pause>();
    commands.spawn(lease); // Despawning this owner releases its demand.
}
```

Continue consuming ready translations through native `Res<Scope>` and the
readiness helpers below. Each `hold` creates an independent owner, including
repeated holds of the same scope; overlapping groups and leaves share loading.
Dropping a token on any thread releases only its owner's demand at the next
publication boundary. Explicit `load` requests survive token drops, and `unload`
releases only the explicit request, leaving automatic consumers and other leases
active. Dropping before the first publication starts
no I/O. Tokens retain demand across language changes and do not keep a removed
controller alive. They cannot be cloned; an application may share one token
through `Arc`, releasing it when the last clone drops. Lazy bindings and
localized required-resource systems own scopes automatically; they do not need
application-held lease tokens. Explicit Manual remains manual. Failed attempts
retry through `set_locale`, `load` or `ReloadCatalogs`; acquiring another lease
does not retry a failed attempt.

`load::<texts::Translations>()` requests the whole tree. It removes the memory
benefit of partial loading while that request remains active. Subdivide very
large translations into useful FTL files: a leaf is parsed and retained as a
whole, not one message at a time.

After the last automatic consumer, explicit request or lease is released,
the plugin drops its scope
snapshots and strong asset handles at synchronization. Bevy may retire assets over
later updates. Application-owned clones can keep data alive, and explicitly embedded
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
While a required scope is absent, the system waits without blocking the frame.
It resumes when its catalogs are ready. During a locale switch, it continues
using ready active resources until the target is published. A complete parent
waits for all its children; a HUD leaf does not wait for the pause module.
In Lazy, the helper requests and retains the required scopes from system
initialization until the system state is dropped. This applies even when an
additional `run_if` is false. Explicit Manual still needs a lease or manual
`load`/`unload`; Full
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
Bevy deferred commands. In Lazy it releases its temporary scope ownership after
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
advanced forms, retain the needed scopes with manual pins or leases and provide
readiness conditions separately; explicit Full is also available. Install the localization plugin before the schedule is
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
`ReloadCatalogs` to retry requested leaves. Repeating `load::<Scope>()` in Lazy
or Manual also retries failed active and prepared leaves while keeping its manual
pin. Repeating `set_locale` retries failures in that requested language without
pinning scopes or reloading already-ready active leaves.

### Observe loading progress

One root progress plugin provides native views for the entire registered tree:

```rust,ignore
use bevy_fluent_typed::LocalizationProgressPlugin;

app.add_plugins(LocalizationProgressPlugin::<texts::Translations>::new());
```

This includes `LocalizationProgress<texts::presentation::Hud>` automatically;
no separate HUD registration is needed. To observe only a group's subtree instead,
use `LocalizationProgressPlugin::<texts::Presentation>::new()`. It includes
that group and its registered descendants, excluding ancestors and siblings.
A leaf plugin observes only that leaf. Consumer plugins may register overlapping
subtrees in separate calls; repeats are idempotent and share one provider dispatcher.

During App setup, either order relative to the owning `LocalizationPlugin` works.
Lazy/Full/Manual is inferred from that base plugin; no Cargo progress feature is required.
Selected scopes get native resources before `Startup` once the base plugin is
installed. Registration snapshots any active/prepared state already present
during setup; compose these plugins before the App is finished. Read a typed view
from an existing UI system, or gate a dedicated observer on visible changes:

```rust,ignore
use bevy::ecs::schedule::common_conditions::resource_changed;
use bevy_fluent_typed::LocalizationProgress;

fn loading_ui(progress: Res<LocalizationProgress<texts::presentation::Hud>>) {
    let active = progress.active();
    println!("{}: {}/{} ready", active.locale, active.ready, active.total);

    if let Some(target) = progress.preparation() {
        println!("target {}: {}/{} ready", target.locale, target.ready, target.total);
    }
}

app.add_systems(
    PostUpdate,
    loading_ui
        .run_if(resource_changed::<LocalizationProgress<texts::presentation::Hud>>)
        .after(bevy_fluent_typed::LocalizationSystems::Progress),
);
```

Without a progress plugin, there is no native progress resource, no recurring
tracker and no tracking identity or revision updates. Ordinary loading and
`status` continue normally. The progress plugin creates neither logging nor a loading UI;
presentation belongs to the application. `active()` covers ordinary initial loading,
`set_locale` and same-language reloads. `preparation()` is present during an
automatic language switch or explicit `prepare_locale`, until cancellation or
publication. Preparing the active locale mirrors
its active data without additional I/O.

The root view counts the current demand union: Lazy includes automatic consumers,
manual pins and leases; Manual includes manual pins and leases; Full counts the
whole root. Group and
leaf views count their fixed, deduplicated schema paths, including unrequested
leaves. Overlapping owners do not inflate the total. Progress plugins and resource
reads create no loading requests or leases and do not retain parsed catalogs.
Each `LoadingProgress<Locale>` contains `locale`, `total`, `ready`,
`loading`, `failed`, `unloaded` and `available`. The four attempt counts sum to
`total`; `available` separately counts usable last-good snapshots, including
during a loading or failed reload. These are module counts, not downloaded bytes
or a percentage of elapsed work.

`active()` and `preparation()` counters describe the observed scope, but
`preparation_status()` is provider-wide because locale commit applies to the
whole provider's demand. A ready HUD does not mean other requested leaves are
ready; conversely, an unrequested observed scope can remain unloaded while the
provider reports `PreparationStatus::Ready`. Check that provider-wide status
before asking the live controller to `commit_locale`. Scoped `ready == total` is
insufficient; retries or asset-handle retirement can also prevent a commit.
`set_locale` switches automatically when the provider is ready; explicit
`prepare_locale` leaves publication to the application through `commit_locale`.
Reading a snapshot never initiates a commit. Manual preparation failures appear
through `PreparationStatus::Failed` without precommit `Rejected` notifications;
automatic target failures emit `Rejected` for that target. Active translations
remain usable throughout preparation.

The tracker runs after PreUpdate publication and after PostUpdate text refresh,
in `LocalizationSystems::Progress`. Read fresh snapshots in `Update`, or order a
PostUpdate observer after that progress boundary as shown above. Use
`resource_changed` to update a loading UI only when visible progress changes;
settled frames preserve its change tick. Several transitions between observations
may coalesce: this is the latest snapshot, not a history of messages. Use
`CatalogUpdateReader` when you need accepted/rejected load notifications.

For passive counts of another scope, `localization.progress::<Scope>()` and
`ModuleStore::progress::<Scope>()` include all unique schema leaves of that scope,
including unrequested leaves. `localization.preparation_progress::<Scope>()`
returns target counts when preparation exists. These on-demand queries create
no demand, enable no recurring tracking and require no optional feature.

See the [generated progress example](https://github.com/SDA-31/bevy_fluent_typed/blob/main/examples/codegen/src/bin/progress.rs) for a bounded headless runner,
explicit prepare/commit and a change-gated observer.

### Optional module details

Enable the default-off `diagnostics` feature only when you need individual
module paths, attempt errors or snapshot availability:

```toml
bevy_fluent_typed = { version = "0.3.0", features = ["codegen", "diagnostics"] }
```

Keep the build dependency unchanged. During explicit inspection:

```rust,ignore
let modules = localization.diagnostics::<texts::presentation::Hud>();
let target_modules = localization.preparation_diagnostics::<texts::presentation::Hud>();
```

`diagnostics::<Scope>()` is also available on `ModuleStore`. Each call collects
sorted `ModuleDiagnostic` values with `path`, `status` and `usable`, including
unrequested schema leaves; target inspection returns `None` without preparation.
The queries neither request nor retain parsed catalogs. Detailed lists and their
error copies are collected only when requested, never by the registered progress
publisher, even with the feature enabled. Counter scans borrow attempt states.
Typed `PreparationStatus::Failed` still retains the first target failure for commit
readiness. The `diagnostics` feature does not activate native tracking; use
`LocalizationProgressPlugin<Scope>` when a native view is needed. Counters and typed
preparation readiness require no optional feature. Details add no dependencies
and start no logging.

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
the current resource and updates the existing `Text`, `Text2d` or `TextSpan`
component when the language, module or binding changes:

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
module loads. In Lazy, insertion requests the HUD and removal releases that
binding's demand. Your application owns the usual Bevy UI/camera/font setup.
Missing or unloaded resources clear bound text, and it
refreshes when the resource becomes available. Bind to the smallest scope the
message uses: a `LocalizedText<Translations>` would wait for the entire tree.

Bindings can be cloned and used in native Bevy 0.19/0.20 scenes; see the
[BSN recipe](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/bsn.md) for the syntax of each backend and child text spans.
A span binding owns its scope just like a root binding in Lazy. Its parent
`Text`/`Text2d`, hierarchy and styles belong to the application; refreshing a
span does not insert a root `Text`. All three targets keep their active-language
content while a new locale loads or fails validation.

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

Call it from your language-selection action. The plugin loads and validates the
currently needed scopes in Spanish while `locale()`, native resources, controller
views and bound text keep using the active language. When all target leaves are
ready, it automatically publishes them together at the next PreUpdate
`LocalizationSystems::Publish` boundary. Text refreshes in PostUpdate. No separate
`commit_locale()` call is needed. Consumers added or removed during acquisition
update the target demand as well.

`prepared_locale()` reports the pending target; `preparation_status()` reports
`Preparing`, `Ready`, `Failed(error)` or `Idle`. A failed target leaves the active
language usable and emits `CatalogUpdate::Rejected` with the target locale. It
is not retried every frame. Repeat `set_locale(target)`, retry a failed requested
scope with `load::<Scope>()`, or send `ReloadCatalogs` after correcting the source.
The latest requested language wins; selecting the active language cancels a
pending target and retries only failed active leaves, without adding manual pins
or reloading ready leaves. Repeating a pending target does not duplicate loads.

For a different initial language, insert `AppLocalization::new(texts::Locale::Es)`
before installing the plugin. Selecting a locale before the first app update
also chooses only that initial language. Otherwise the manifest default is used.
No implicit fallback locale or retained locale cache is introduced.

### Advanced: prepare and choose the publication time

Use explicit preparation only when the application must decide when a validated
target becomes active, for example at a transition boundary:

```rust,ignore
use bevy_fluent_typed::PreparationStatus;

localization.prepare_locale(texts::Locale::Es);
// In a later update, publish only when the application is also ready to switch:
if let PreparationStatus::Ready = localization.preparation_status() {
    localization.commit_locale()?;
}
```

`prepare_locale` takes over manual control, including for a target already
requested with `set_locale`: it stops automatic switching. Preparation follows
current automatic consumers, manual pins and leases in Lazy, pins and leases in
Manual, or the whole language in Full, using the existing source and checked parsers.
Target leaves remain private until commit. `commit_locale()` returns
`CommitLocaleError::NotPrepared`, `Pending` or `Failed(ModuleError)` until every
requested target leaf passes its latest attempt. A retained earlier good target
snapshot does not satisfy a failed retry. Success queues the coordinated swap
for the next PreUpdate Publish boundary; committed leaves emit `Loaded`.
Manual preparation failures remain available through `preparation_status()`;
they do not emit `Rejected`. Automatic `set_locale` failures emit `Rejected`
for their target before switching.

Repeating `prepare_locale` keeps successful leaves and retries failures; a
different target replaces it. Changing the required leaf union, or explicitly
retrying leaves, revokes a queued manual commit. Poll readiness and commit again.
Publication rechecks source readiness. Automatic `set_locale` intent instead
continues through demand changes and explicit retries, and commits when ready.

`cancel_preparation()` cancels both manual preparation and pending automatic
switching, dropping target snapshots and tasks/handles. External I/O may finish
later, but canceled attempts cannot publish a replacement target. Preparing the
active locale is immediately ready and performs no additional I/O. An empty
Lazy/Manual demand also needs no module acquisition.

Preparation retains active and target parsed leaves until cancellation or commit.
Application-owned clones and source buffers retain their usual lifetime. File
sources use Bevy's AssetReader and AssetLoader with private preparation assets
and per-attempt identities. A retry may wait asynchronously for obsolete handles
to retire; uncancelable readers or another owner retaining a handle may delay
readiness. Active consumers keep running. Source metadata must allow Bevy to
select the requested asset type; `.meta` files selecting another loader cause
preparation to fail. Remove that override or configure the source's metadata
policy. Watching and `ReloadCatalogs` continue after commit. Preparation does not
snapshot a changing archive or order overlapping external reloads; keep the
source coherent, as described below.

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
`set_locale` prepares the target while leaving the active controller locale and
resources usable. A ready target switches at PreUpdate Publish, then bindings
refresh in PostUpdate. Consumers that need the refreshed text in that frame
should run after `LocalizationSystems::Refresh`.

Checked leaves publish independently. A bad same-language reload keeps that
leaf's last good value; valid siblings can still change. Ordinary same-language
reloads have no multi-file transaction. Locale replacement commits its currently
requested target leaves together after validation, automatically with `set_locale`
or at the application's chosen time with `prepare_locale`/`commit_locale`.
Successful reloads publish fresh snapshots even if text is identical;
idle frames and unchanged siblings preserve resource identity and change ticks.

The plugin serializes its own reload requests per module and coalesces pending
retries. Watcher reloads and direct `AssetServer::reload` calls bypass that queue.
Bevy exposes no request generation before opening the reader or in a failure
event, so overlapping external reloads cannot always be ordered by request time.
If strict order matters, disable automatic watching and send `ReloadCatalogs`;
a custom reader must supply coherent data as well.

## Custom byte sources

For on-demand reading and unloading, use [Lazy](#automatic-module-lifetime)
with Bevy's asset system, or [manual loading](#manual-loading). [Custom asset sources](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/asset-sources.md) let the
application supply transport, decompression and caching through an `AssetReader`.

`from_bytes` is for data the application has already obtained and intends to keep
in memory. **The plugin retains all supplied buffers for later loads and language
changes.** In Lazy, only parsing and resource creation wait for consumers;
removing consumers releases unneeded parsed resources, but retains input buffers.
Explicit Manual follows manual requests with the same buffer lifetime.

Keep `codegen`, `build.rs` and `translations!`. Generated accessors work with
these buffers too; only the plugin constructor changes. This path needs
`MinimalPlugins` or `DefaultPlugins`, without `AssetPlugin`.

For the guide's two English modules, read both files and keep the default Lazy:

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
the application. Reload notifications and explicit retries use the same
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
The plugin still manages automatic consumers or explicit Full/Manual requests,
typed resources, readiness, bindings, language changes, retries and unloading.

Follow the [complete handwritten example](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/no_codegen) for these
implementations. Add `manifest` if that provider needs the existing manifest/file
constructors. `manifest` is enabled by default and by `codegen`; it uses the
generator package's small runtime manifest API, without enabling generation.
See the [feature table](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/build.md#features) for host/target separation.

For advanced handwritten providers with groups, declare the immediate parent
in each child descriptor:

```rust,ignore
ScopeRegistration::new::<Hud>().with_parent::<Presentation>()
```

The parent must share the child's `FluentScope::Catalog`. Descriptors without a
parent are direct children of the catalog root; root observation includes every
registered descriptor, including existing providers without parent metadata.
Generated Bevy providers emit these relationships automatically from their accessor
hierarchy. The one-file handwritten example needs no parent boilerplate.

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
initialization, readiness handling, optional Manual adoption and handwritten providers.
The standard upgrade uses Lazy with inserted bindings and localized systems;
choose explicit Full to preserve eager loading or direct world polling.

## Compile-time mode boundaries

`Lazy`, `Full` and `Manual` implement the sealed `LoadingMode` trait. Explicit
requests and independent leases are available on Lazy and Manual. The compiler
rejects requests on Full and scopes
belonging to another root:

```compile_fail,E0599
use bevy_fluent_typed::{FluentCatalog, Full, Localization};
fn request<C: FluentCatalog>(state: &mut Localization<C, Full>) {
    state.load::<C>();
}
```

```compile_fail,E0599
use bevy_fluent_typed::{FluentCatalog, Full, Localization};
fn hold<C: FluentCatalog>(state: &mut Localization<C, Full>) {
    let _lease = state.hold::<C>();
}
```

```compile_fail,E0599
use bevy_fluent_typed::{FluentCatalog, Full, Localization};
fn release<C: FluentCatalog>(state: &mut Localization<C, Full>) {
    state.unload::<C>();
}
```

```compile_fail
use bevy_fluent_typed::{FluentCatalog, FluentScope, Manual, Localization};
fn wrong_root<C: FluentCatalog, S: FluentScope>(state: &mut Localization<C, Manual>) {
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
