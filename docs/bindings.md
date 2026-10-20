# Application-defined localization bindings

Use `Localized<Target, Scope>` for every automatic binding: ordinary Bevy text,
application widgets and semantic labels. Targets implement `Component`; `Default` is optional.
The binding owns `Msg`; the target component stores its output. All targets use the same
provider, loading modes and entity-owned demand, without a user refresh system
or component registration.

`Msg<Scope>` is the deferred formatter type. `Message<Scope>` remains a deprecated
alias with the same constructors and binding conversions; use `Msg` in new code.

## Bind a custom component

Implement `LocalizationTarget` on your component. Add the normal source plugin
from the quickstart, then spawn the component with its binding:

```rust,ignore
use bevy::prelude::{Component, EntityWorldMut};
use bevy_fluent_typed::{LocalizationTarget, LocalizationUpdate, Localized};
use texts::presentation::Hud;

#[derive(Component)]
struct MyCaption {
    text: String,
}

impl LocalizationTarget for MyCaption {
    type Data = &'static mut Self;
    type Filter = ();

    fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
        let Some(target) = entity.get_mut::<Self>() else {
            return;
        };

        update.set_text(target.map_unchanged(|target| &mut target.text));
    }
}

commands.spawn((
    MyCaption { text: String::new() },
    Localized::<MyCaption, Hud>::new(|hud| hud.msg_title()),
));
```

Only Data/Filter and `apply` are required. This example uses the default sequential
writer; no registration, additional plugin or derive helper is needed. For direct
query writes and Idle skipping, override `update` and `needs_update` as shown by
Badge in the [runnable example](https://github.com/SDA-31/bevy_fluent_typed/blob/main/examples/bindings/src/widgets/targets.rs).

Implementing the trait alone does not opt entities into automatic updates.
Each entity needs its own `Localized<Target, Scope>` binding; native text uses
the same rule as application components.

Spawn the target and its binding together. When the deferred add callback runs,
a missing target removes the binding with one warning naming the entity, target
type and scope. No component is created automatically, even when it implements
`Default`. Rejected bindings do not retain their catalog or formatter captures.
Warnings use standard tracing/Bevy logging and do not require the diagnostics feature.
The check does not require a ready catalog and does not evaluate Data/Filter.
Applications also supply any neighboring components used by the adapter.

Supplied components keep their settings. Removing the target after a successful
attachment does not recreate it, including after a language or Msg change.
Reinserting the target resumes updates. Removing and adding the binding checks
presence again; replacing its Msg does not. A rejected binding does not revive
when a target appears later: attach a new binding alongside that target.

Compound and immutable components use this same trait. Components without
`Default` are supported; non-component field markers are not. Other hooks have defaults:

| Hook | Default and purpose |
| --- | --- |
| `needs_update(&data, &update)` | Returns true. An optional pure predicate can skip Idle work and guide parallel scheduling. |
| `update(data, &mut update)` | Returns `UpdateResult::Deferred`, requesting sequential `apply`. Override for direct query writes. |
| `apply(entity, &mut update)` | Required writer. Read `update.text()` only when a translation is needed. |
| `matches(read_only_data, text)` | Returns false. Optional confirmation of a deferred write after the whole batch and its hooks. |
| `PHASE` | `TargetUpdatePhase::BeforeLayout`. |

`UpdateResult::Done` means handled, including an unchanged value or an intentional
wait. `Deferred` requests `apply` after query borrows end. Neither step forces a
message read. With `CachePolicy::None`, `text()` formats on demand and reuses that
value for the rest of the invocation and its deferred continuation. Rendered
retains prepared output instead. A context's invalidation remains pending until
it is read. Returning false from `needs_update` postpones that work; if the catalog
changes while paused, resuming uses its latest value without intermediate writes.

`update.is_changed()` covers catalog/readiness, Msg identity and query reentry.
Combine it with `update.component_changed(&component)` for relevant Ref/Mut data.
This tracks application changes while excluding confirmed own writes. Ordinary
Bevy `is_changed()` remains available with its normal semantics. Deliberate
`bypass_change_detection` writes are outside the built-in tracking.
Compare before writing to preserve target ticks. For a String field, the shared
`update.set_text(target.map_unchanged(|target| &mut target.text))` helper performs
that check, marks actual writes and can reuse buffers when a translation grows.
It leaves `update.text()` available for additional outputs. The built-in mutable text adapters and the Badge example use this same public helper.

`matches` receives the read-only view of the same Data/Filter query and must be
pure. Return true only when the relevant outputs equal the supplied text. The
engine checks after all deferred callbacks and their hooks, then acknowledges
its own writes. A skipped structural write stays pending even if its filter
returns before the next refresh; it is retried when eligible. The default is correct without confirmation, but can cause an
additional read on the next pass under None. Users do not implement a tracking
component, private cache or refresh system.

The scope snapshot is cloned before borrowing Data, including on modern Bevy's
resource entities. Callback rows are disjoint and may run on compute workers;
do not depend on thread identity or entity order. Neither callback may change
its own binding; `apply` must not despawn the entity. An apply-only adapter queues
work every eligible refresh. Under None it formats every time it reads text.

## One Query and native filters

`Data` can contain several neighboring components. `Filter` accepts `()`, `With`,
`Without`, tuples and nested `Or`. For example, part of a paired adapter could be:

```rust,ignore
type Data = (
    Option<Ref<'static, Caption>>,
    Option<&'static mut SemanticNode>,
);
type Filter = (
    With<Widget>,
    Without<Editing>,
    Or<(With<MenuItem>, With<OverlayItem>)>,
);
```

Every item belongs to the binding's entity. The engine uses one native query for
that adapter, composing its own `Entity` and `Localized` access with your Data.
Do not request the binding itself or whole-entity mutable access, which would
conflict with that access. Bevy validates query access normally. On Bevy 0.19/0.20,
Data must implement native `SingleEntityQueryData` to preserve disjoint rows;
older backends use their native `QueryData`. There are no auxiliary user Query
parameters or arbitrary SystemParam hook in this contract. Application systems
can compose relationships across entities; independent bindings can share Msg
clones when several targets need the same translation.

`'static` describes reference types in Data; the actual query items borrow the
running query, not the program's entire lifetime. `TargetItem` names that actual
item consistently across all supported backends, including Bevy 0.16's different
native `QueryItem` lifetime arity. This follows Bevy's
[QueryData model](https://docs.rs/bevy_ecs/latest/bevy_ecs/query/trait.QueryData.html).

Required `&mut Caption` or `Mut<Caption>` excludes an entity without Caption,
so updates wait while the component is absent. `Mut<'static, Caption>` additionally
preserves a Ref in the query's read-only view, including change detection.
Built-in targets and the Badge example use application-owned components and required
data without an Option.
Use optional data only when absence is part of your custom logic, such as an
optional neighbor; it does not itself create or retain anything.

Filter is an `ArchetypeFilter`: `Changed`/`Added` are not accepted, including
inside tuples or `Or`. A language or Msg can change without changing the target
first. Observe change ticks inside Data instead. Excluded entities keep their current
output; reentry after an excluded refresh catches up to the current translation.
A filtered-out binding still retains its catalog in Lazy. To release that demand,
remove its binding or despawn the entity.

## Supported targets

Implement `LocalizationTarget` directly on a component; no `Default` implementation
is required. Use one component type per independently bound value. Non-component
field markers and separate adapters for foreign component types are not supported.

`Localized::<MyCaption, Hud>::from(message)` accepts an existing Msg.
Its scope can also be inferred: `Localized::<MyCaption, _>::from(message)`.
Replace the companion binding when the message or its captured arguments change.
A Msg stored elsewhere does not become a subscription: replacing that field
does not replace the companion, even if they originally shared a clone.

Built-in adapters are `Text`, `Text2d` and `TextSpan`, plus `AccessibleLabel` on
Bevy 0.19/0.20. Like application components, each must be supplied alongside its
binding. TextSpan still needs an application-owned text parent. A custom target
never receives an implicit UI Text from the library.

## Semantic labels and refresh ordering

```rust,ignore
let name = Msg::<Hud>::new(|hud| hud.msg_title());
commands.spawn((
    AccessibleLabel::default(),
    Localized::<AccessibleLabel, Hud>::from(name),
));
```

This updates the same entity's AccessibleLabel, including icon-only controls.
Its immutable replacements run Bevy's ordinary insertion hooks. Bevy owns
AccessibilityNode and its required-component behavior: a normal AccessibleLabel
insertion can create that node and update its name. The localization runtime does
not independently query, repair or recreate the node. Node-only edits/removal do
not invalidate Msg. Other UI producers may overwrite the node later, just as
with an unbound AccessibleLabel; coordinate native producers or implement a
custom paired adapter if your application needs a different policy.
The application owns accessible roles, tree, focus and interaction.

Targets default to `TargetUpdatePhase::BeforeLayout`, in
`LocalizationSystems::Refresh`. AccessibleLabel uses `AfterLayout`, in
`LocalizationAccessibilitySet`, after UI accessibility producers
and before accessibility export. Custom adapters may select the
same phase through `LocalizationTarget::PHASE`. On Bevy 0.16–0.18 there is no
AccessibleLabel adapter; AfterLayout custom callbacks follow the localization
refresh, without a UI-producer/export ordering guarantee.

## Choose a provider's cache policy

Configure the owning root provider once; its leaf and group bindings inherit
the policy. Independent root providers can choose differently.

```rust,ignore
use bevy_fluent_typed::{CachePolicy, LocalizationPlugin};

app.add_plugins(
    LocalizationPlugin::<texts::Translations>::new(HUD)
        .with_cache_policy(CachePolicy::None),
);
```

`HUD` is the generated embedded manifest from the quickstart. The same builder
works with files, `from_bytes` and `from_loader`, in Lazy, Full and Manual.
There is no per-binding override or runtime policy switch.

| Policy | Retained output | Repair and Idle behavior |
| --- | --- | --- |
| `CachePolicy::Rendered` (default) | An additional formatted String per binding, in the same ECS metadata for all targets. | Target repair reuses that output. Unchanged native text does not reformat. |
| `CachePolicy::None` | No formatted output retained by the runtime; identity and invalidation metadata remain. | Built-in targets skip Idle and format for application edits or invalidation. Custom adapters format when they request `update.text()`. |

Rendered and built-in targets format after catalog/Msg changes and eligibility
reentry. An uncached custom adapter sees that invalidation through
`update.is_changed()` and requests text when it can handle it. Equal output
avoids native target writes. Disabling the result cache does not discard
the String owned by Text, a custom component or AccessibleLabel, nor the name
owned by AccessibilityNode; these are the target values. Deferred work can keep temporary output through its callback and confirmation;
None retains none of that output after the refresh.

A custom adapter can inspect query or entity data before asking for text. An
intact caption can repair a companion from its own String without Fluent; the
Caption/CaptionMirror example combines this with a structural pause filter.
An apply-only adapter that always reads text formats every eligible refresh
under None. Built-in and manual targets share the same optional
post-batch confirmation for immutable/structural writes.

Cache policy does not change module loading, scope lifetime, readiness, locale
publication or `Msg::render`, which always invokes the formatter immediately.
Native text mutations through `bypass_change_detection` remain deliberately
untracked under both policies. The [binding example](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/bindings)
can run the same scenario with and without retained results.

## Loading and lifecycle

Creating or cloning an unattached binding/message is passive. Lazy retains
inserted companions, alongside other consumers, pins and leases. A target
component without a binding does not request a catalog. Full requests all modules; Manual still requires explicit
`load` calls. When a catalog is unavailable, adapters receive an empty string. Preparing a target locale keeps active values
until publication, including preparation failure.

Catalogs remain immutable snapshots. Capture owned arguments, not a translated
string or catalog snapshot. Number formatting remains application-owned; use
locale-aware ICU/ICU4X services inside the formatter. Changing captured external
state does not automatically invalidate a Msg: replace it when inputs change.

Native BSN supports `Localized::<Target, Scope>::new(...)` and `from(message)`
on Bevy 0.19/0.20; missing formatter constructors fail during scene construction.
See [button composition](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/bsn.md#buttons-and-caption-slots).

<a id="every-message-rendering-and-update-path"></a>

## Every Msg rendering and update path

A Msg stores `Fn(&Scope) -> String`. There are two entry points:

| Entry point | Who invokes the formatter and applies its result |
| --- | --- |
| `Localized<Target, Scope>` | The runtime formats according to the provider's cache policy and updates the target on the same entity. No target registration is needed. |
| `message.render(&scope)` | The caller invokes the formatter immediately on every call, without this runtime cache, and decides where to put the String. |

The built-in targets are UI `Text`, world `Text2d`, child `TextSpan`, and
`AccessibleLabel` on Bevy 0.19/0.20. Custom targets may address an entire component
or a field. `Text` does not imply an AccessibleLabel, and a parent binding does
not propagate to text children: each target needs its own binding.

World spawning, Commands, later insertion, native BSN, Scene and SceneList
composition all use this same binding path. Localized scene templates and
`Msg::new`, `Msg::literal`, `Localized::new`, `from` and `clone` are
passive. Formatting starts after insertion and deferred commands, when a matching
provider enables that scope and its refresh job runs. Preexisting instances are
adopted during App setup without target registration.

### When the automatic formatter runs

The following table describes built-in targets under the default
`CachePolicy::Rendered`. All use the [shared ECS state](https://github.com/SDA-31/bevy_fluent_typed/blob/main/src/target_bindings/state.rs)
and [common engine](https://github.com/SDA-31/bevy_fluent_typed/blob/main/src/target_bindings/engine.rs).
Custom adapters additionally decide when their callbacks read text. An eligible
row needs its required target data; removed components receive no refresh.
A ready scope is required to call Msg; otherwise the supplied output is empty.
Caches are separate per World, binding type and entity, without deduplicating
owners of the same cloned Msg.

| Situation observed by refresh | Formatter invocation | Output behavior |
| --- | --- | --- |
| First eligible target with a ready scope | Yes | Apply the translation. |
| Binding attached to a supplied target before its scope loads, or the first load fails | No | Eligible output is empty. |
| Target missing when the binding is first attached | No | Remove the binding with a warning; create no component or catalog demand. |
| Scope becomes ready after loading, retry or reacquisition | Yes | Publish the current translation. |
| `set_locale` publishes a prepared language | Yes | Keep the active language until publication, then update. |
| Manual `commit_locale` reaches publication | Yes | The call queues the commit; resource publication invalidates bindings. |
| Valid same-language reload publishes a fresh scope, including identical bytes | Yes | Equal output preserves target ticks. |
| A new Msg identity replaces the existing binding's formatter | Yes, when ready | Recompute without running initialization again. |
| A cloned Msg is attached to another entity or target | Yes, when ready | That binding has independent metadata and output. |
| Existing binding is replaced by a clone of its current Msg | No, if inputs are unchanged | Reuse output without initialization. |
| Binding removed and added again | Yes, when eligible/ready | Check target presence again; reject a missing target with a warning. |
| Scope becomes unavailable | No | Clear eligible output; missing targets stay absent. |
| Existing target is edited through ordinary Bevy mutation | No additional formatting | Repair from cached output. |
| Target is removed while Localized remains | No | Respect removal; catalog and Msg changes do not revive it. |
| Target/query eligibility returns after an excluded refresh | Yes, when ready | Catch up to current inputs. |
| Native output is edited through `bypass_change_detection` | No | Leave the untracked edit until another observed change or invalidation. |
| AccessibilityNode alone is edited or removed | No | Leave it to Bevy/application behavior; do not independently repair it. |
| No relevant input or target change | No | Skip work and preserve target ticks. |

Under None, eligible existing target edits and the invalidations above request
a fresh translation. Missing targets still remain absent and do not format.
Repeated text reads within update/apply reuse the invocation's value. Shared
post-batch verification excludes confirmed own writes from component_changed;
external writes from later hooks remain detectable. An unavailable scope never
calls Msg under either policy.

Scope changes are tracked at the selected scope's granularity, not per Fluent
key. Reloading another key in the same leaf still invalidates a Msg using
that leaf; a group/root also follows changes to any required descendant.
Missing published scope resources are restored from the retained snapshot,
which marks them changed as well. Several changes before one refresh can coalesce;
the formatter reads the state visible at that refresh.

`prepare_locale`, progress changes, loading attempts and rejected reloads do not
by themselves invoke a Msg. Active catalogs stay available during a pending,
failed or cancelled language switch and failed same-language reload. Independent
active-catalog edits or Msg replacements can still update text during that wait.
Selecting the already active locale does not reload ready leaves; it can retry
failed ones, whose later successful publication uses the normal rules above.

### Applying a cached result is separate work

The runtime checks active bindings in PostUpdate through one query composed
from Data, Filter, Localized and shared ECS metadata. It uses the same callbacks
for Text, Text2d, TextSpan, AccessibleLabel and user targets, without known-type
dispatch or a private target protocol.

Small sets stay sequential. Large sets with an unchanged catalog inspect dirty
work first; a few changes stay sequential, large updates use Bevy's parallel
query iterator, and all-Idle sets avoid task scheduling. Query size hints provide
a scheduling estimate without a separate row-count traversal. This still reads
binding metadata each frame; it is not zero-cost polling. String comparison
before writes preserves equal-output ticks and reuses target buffer capacity.

`needs_update` is a pure optional predicate and may be checked during scheduling.
`update` receives its row once when selected. Done includes unchanged output or
waiting; Deferred queues sequential apply after borrows end. The lazy context
allows apply to use existing neighbor data without a forced Msg read. Membership
and Msg identity are rechecked before each structural callback. The same query's
read-only view verifies opt-in matches only after the whole batch and hooks.
No user tracking component or second refresh system is required.

The initial presence check is separate from refresh and happens only on binding
attachment. It preserves supplied settings and removes a binding whose target is
absent, without creating tracking state. Removing a target later does not run the
check again; removing Localized retires its metadata, while an existing target stays intact.
Temporary query exclusion can retain cached storage and binding-owned catalog
demand; reentry uses current inputs.

Parallel query work requires Bevy's native `multi_threaded` feature; a multi-worker
pool alone does not enable it. Work completes before this refresh returns. There is no built-in frame
budget or automatic spreading across frames. Formatting and component updates
are only part of a language switch's cost: Bevy must still process changed text
and layout. The runtime does not create a separate worker pool.
See the [performance reference](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/performance.md)
for formatter counts, cache tradeoffs and measured refresh costs.

BeforeLayout targets run in `LocalizationSystems::Refresh`. AfterLayout targets
run in `LocalizationAccessibilitySet`; the AccessibleLabel adapter uses that
phase after UI accessibility producers on Bevy 0.19/0.20. Insertions or user
writes ordered after the relevant phase take effect on its next refresh.

Formatting does not run on an independent timer or directly on a file-watcher
thread. Files, embedded manifests, supplied bytes and asynchronous/custom asset
sources all enter the same scope publication and refresh path. `ReloadCatalogs`
requests a fresh read; changing a source buffer without a read/publication does
not invalidate a Msg. Source validation and successful publication remain
required for automatic source-driven updates.

Full, Lazy and Manual choose module loading and lifetime. All three still refresh
existing bindings automatically when a scope is available. Manual means explicit
module requests; application-owned text uses a plain String without a binding.
Applications construct the targets and supply the deferred Msg explicitly.

### Cases that do not establish automatic ownership

A standalone Msg in a variable or a field of a resource, event or target component
has no automatic refresh. A Msg is not itself a Bevy component. Reading
`Res<Scope>` or calling a generated `scope.msg_hello()` returns application-owned
data; it does not attach a binding to any resulting String. A direct
`message.render(&scope)` is likewise a caller-owned snapshot. Running that call
in Update, Startup, an observer or another caller only changes when the caller
runs; each explicit call formats again.

Changes to other resources, captured Arc contents, numbers or formatter settings
are not inferred. Replace the Msg when these inputs change. A closure that
captures an already translated String keeps that snapshot, even when called
with a newer scope. `Msg::literal` intentionally returns fixed text, but in
an automatic binding it still needs a ready scope; without one the output is empty.

Removing a `Localized` companion stops its writes and leaves an existing target's
last String in place. Removing only the target leaves Localized attached but does not recreate the
component. Its required query data is absent; reinsert the component to resume.
The binding still owns its catalog demand until removed.
Despawn removes the entity through Bevy. Remaining bindings, native consumers,
pins, leases or Full can keep catalogs loaded; an unattached Msg alone does not.
Use separate component types for independent bindings. Each bound component
should have one localization writer. An application marker such as UserOwned does not
change these rules; ownership follows the actual binding types.

### Hidden and disabled entities

Visibility, layout and styling do not pause these refresh queries. A hidden text
still follows its scope. Bevy's default query filters exclude Disabled entities;
they receive no refresh while excluded. Returning after at least one excluded
refresh causes a fresh formatting pass when a scope is available.

`Localized` hooks count the inserted binding even while Disabled, retaining
its Lazy demand. Reenabling resumes refresh; disabling is not an unload request.
Removing the binding or despawning releases its ownership. Merely hidden entities
continue both refresh and demand as usual.

## Runnable examples

The [complete binding example](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/bindings)
keeps native targets and custom components in one focused package:

```sh
cargo run --locked --manifest-path examples/bindings/Cargo.toml
```

| Task | Starting point |
| --- | --- |
| Configure the source | [main.rs](https://github.com/SDA-31/bevy_fluent_typed/blob/main/examples/bindings/src/main.rs): ordinary source plugin and Startup system |
| Spawn native text, a button and custom components | [widgets.rs](https://github.com/SDA-31/bevy_fluent_typed/blob/main/examples/bindings/src/widgets.rs): explicit targets, custom adapters, styling and hierarchy |
| Implement component adapters | [targets.rs](https://github.com/SDA-31/bevy_fluent_typed/blob/main/examples/bindings/src/widgets/targets.rs): Title, Badge and Caption |
| Change arguments, switch EN/ES/RU and release demand | [scenario.rs](https://github.com/SDA-31/bevy_fluent_typed/blob/main/examples/bindings/src/scenario.rs): headless driver, separate from spawning |
| Compose Scene or SceneList captions | [BSN buttons](https://github.com/SDA-31/bevy_fluent_typed/blob/main/examples/bsn/src/buttons.rs) and [composition recipes](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/bsn.md#buttons-and-caption-slots) |

The executable runs without a window or font assets. The
[process test](https://github.com/SDA-31/bevy_fluent_typed/blob/main/examples/bindings/tests/bindings.rs)
runs the automatic path under both cache policies, including query-filter reentry.

## Replace the previous text binding

`LocalizedText` has been removed. Use `Localized<Text, Scope>`,
`Localized<Text2d, Scope>` or `Localized<TextSpan, Scope>` according to the target.
The target type is explicit; the scope can be inferred with
`Localized::<Text, _>::from(message)`. A binding does not inspect neighboring
components to guess its target, and it never inserts an AccessibleLabel unless
that target is bound explicitly.
