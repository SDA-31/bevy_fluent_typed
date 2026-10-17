# Changelog

Notable changes to the runtime (and its former companion bridge) are recorded here using
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html);
before 1.0, a minor release can introduce incompatible API changes.

## [0.4.0]

Release preparation; this version has not been published. See the
[migration from 0.3.1](docs/migration-0.4.0.md). The generator remains
`fluent_typed_codegen` 0.2.2.

### Added

- `LocalizationPlugin::validate_registration` returns a typed
  `LocalizationRegistrationError` for repeated root catalogs, with their schema
  leaf paths, both plugin types and constructor locations. Ordinary `add_plugins`
  performs the same structural check and reports the detailed conflict before
  configuring the second provider. Sources, locales and FTL contents are not
  compared. See [registration conflicts](docs/runtime.md#registration-conflicts).
- `Localized<Target, Scope>` and the public `LocalizationTarget` contract bind
  deferred messages to native text and custom components implementing
  `Component + Default`.
  Adapters describe one native Bevy query, with default callbacks and optional
  structural work. Built-in targets use this same contract.
- Automatic `AccessibleLabel` bindings on Bevy 0.19/0.20 through Bevy's ordinary
  accessibility hooks, independently of visible text bindings. The runtime does
  not independently repair or recreate `AccessibilityNode`.
- Provider-wide `CachePolicy::Rendered` (default) and `CachePolicy::None`.
  The latter retains tracking metadata without retaining formatted output.
- Focused custom component examples and native BSN button-caption recipes.
- A [performance reference](docs/performance.md) describing skipped work,
  provider cache tradeoffs and benchmark results against runtime 0.3.1.
- Tracked standalone lockfiles for the runtime, examples and
  compatibility tool, with locked CI graphs.

### Changed

- Large eligible query updates use native Bevy query batching and the existing
  compute pool. Msg formatters and direct query callbacks can run concurrently;
  they must not depend on main-thread identity or entity order. Small updates
  stay sequential; structural callbacks remain sequential after the query pass.
  This is not a frame-budget guarantee or an adaptive cost controller.
- Replacing an existing binding with a clone of its current Msg reuses rendered
  output while the catalog, readiness and query eligibility are unchanged.
- Built-in targets and custom adapters using `LocalizationUpdate::set_text`
  preserve target change ticks when output is equal. Custom adapters can inspect
  component changes before requesting text; uncached text reads are shared within
  an update and its deferred continuation.
- **Behavior change:** built-in text bindings restore translated output after
  ordinary application edits even when Msg and catalog are unchanged. Repairs
  use Bevy change ticks and reuse retained output under the default cache policy;
  explicit `bypass_change_detection` edits remain untracked until another observed
  change or binding invalidation. Idle preserves target ticks.
- **Breaking behavior change:** all targets require `Component + Default`.
  They are initialized once when their binding is attached, preserving supplied
  components and settings.
  Later target removal is respected, including across Msg and catalog changes;
  reinsert the target to resume updates, or remove and add the binding to start
  a new lifetime. Replacing Msg alone does not repeat initialization. In Lazy,
  the binding retains its scope while the target is absent. Removing the binding
  stops automatic writes and leaves the target's last value in place. Custom
  components use the same automatic initialization as built-ins. The optional
  `LocalizationTarget::initialize` hook runs afterward for neighbor setup. See the
  [binding lifecycle](docs/bindings.md#loading-and-lifecycle).

### Deprecated

- `Message<Scope>` is a deprecated alias of the shorter `Msg<Scope>`.

### Removed

- **Breaking:** `LocalizationPlugin::with_startup_strategy` and
  `LocalizationPlugin::with_startup_barrier` on Bevy 0.19/0.20. Configure the
  App-wide policy through `LocalizationStartupPlugin::new` or `::barrier`;
  source plugins inherit it. The implicit Deferred fallback and single
  coordinator are preserved. See the [startup migration](docs/migration-0.4.0.md#configure-startup-through-the-application-plugin).
- **Breaking:** `LocalizedText<Scope>`. Select the destination explicitly with
  `Localized<Text, Scope>`, `Localized<Text2d, Scope>` or
  `Localized<TextSpan, Scope>`, including in BSN scenes.

### Fixed

- Preserve pending catalog invalidation while adapter predicates or query
  filters suspend work, including filter changes during structural callbacks.
- Confirm deferred writes after the whole callback batch and its hooks, without
  hiding later application edits or acknowledging skipped callbacks.
- Preserve native change observation in deferred `apply` contexts, so adapters
  can skip unchanged uncached messages there as well as in query callbacks.

## [0.3.1] - 2026-10-11

See the [migration from 0.3.0](docs/migration-0.3.1.md). The generator remains
`fluent_typed_codegen` 0.2.2.

### Added

- Automatic catalog demand and readiness for ordinary native `Res<Scope>`
  registrations on Bevy 0.19/0.20, including aliases, derived/nested parameters,
  `ParamSet` and catalog reads in run conditions. Recurring consumers retain
  scopes while their system state exists, even when an application condition is false.
- `StartupStrategy` and builders on `LocalizationPlugin` for Deferred, Native
  and selected-set Barrier startup. Multiple roots share one application policy.
- Public `LocalizationStartupPlugin` on Bevy 0.19/0.20 for independent startup
  configuration. Root plugins add an implicit fallback when it is absent;
  explicit startup configuration works in either registration order with one coordinator.
- `PassiveCatalog<Scope>` on Bevy 0.19/0.20 for optional observers that must run
  without requesting or retaining absent catalogs.

### Changed

- **Behavior change:** startup defaults to Deferred on Bevy 0.19/0.20.
  Independent steps execute once while catalog consumers and their declared
  dependents wait; later startup phases follow completion. Frame schedules keep
  running, so systems using ordinary resources from unfinished startup need
  application readiness guards. Select Native to retain ordinary one-shot startup.
- **Behavior change:** plain `Option<Res<Scope>>` in automatically discovered
  systems is treated as a catalog consumer because Bevy does not distinguish its
  scheduler metadata from required `Res`. Use `Option<PassiveCatalog<Scope>>` for
  passive observation. Older native optional reads remain passive.
- Quickstarts and generated resource examples use native `add_systems` on
  Bevy 0.19/0.20. Bevy 0.16–0.18 retain their existing explicit registration
  helpers for direct catalog demand and readiness. Inserted `LocalizedText`
  retains automatic scope ownership on every backend.

### Removed

- **Breaking:** `LocalizationAppExt`, `localized` and `IntoLocalizedSystems`,
  including both recurring and independent one-time registration helpers,
  on Bevy 0.19/0.20. Use ordinary native registration on those backends.
  Bevy 0.16–0.18 retain these helpers; their implementation and exports are
  excluded from newer engines and build-only configurations. See the
  [older-backend recipe](GUIDE.md#older-bevy-backends).

## [0.3.0] - 2026-10-09

See the [migration from 0.2.2](docs/migration-0.3.md). The generator dependency
remains `fluent_typed_codegen` 0.2.2.

### Added

- Opt-in `LocalizationProgressPlugin<Scope>` recursively observes a scope and
  its registered descendants with typed native resources, root demand counts or
  fixed group/leaf schema counts, active/prepared snapshots and provider-wide
  preparation readiness. Root selection includes every registered scope;
  overlapping selections share one provider dispatcher. Generated hierarchy
  metadata is automatic; handwritten providers can declare typed immediate parents.
  Observe after `LocalizationSystems::Progress`;
  no Cargo progress feature or loading request is introduced. Includes a
  [generated-provider example](examples/codegen/src/bin/progress.rs).
- Default-off `diagnostics` for explicit per-module detail queries; it does not
  activate native tracking. Progress counting collects no detail vectors or
  per-module error copies; passive per-scope queries require no registration.

- Optional manual scope pins in Lazy through idempotent `load`/`unload`, without
  releasing scopes still owned by automatic consumers, leases or overlapping pins.
- Advanced locale preparation and explicit commit for application-chosen
  publication time, supporting Lazy/Full/Manual, cancellation and byte/file sources.
- Independent `ModuleLease<Scope>` ownership in Lazy and Manual through
  `localization.hold::<Scope>()`. Dropping a token releases only its owner's
  request at publication; other leases, automatic consumers and explicit requests
  remain active.
- Stable Bevy 0.20 support through `bevy-0-20`, including typed resources,
  readiness, deferred startup, custom asset sources and translation hot reload.
- Cloneable `LocalizedText` bindings and native `FromTemplate` integration for
  Bevy 0.19/0.20 BSN scenes. Typed `new`/`from` constructors work directly inside
  scene blocks; a missing message is an error. Includes a
  [scene recipe](docs/bsn.md) and an executable example covering module lifetime
  and translation updates.
- A UI text default for standalone `LocalizedText` components: refresh inserts
  `Text::default()` only when no `Text`, `Text2d` or `TextSpan` is present.
- `LocalizedText` bindings on `TextSpan` children of UI and world-text roots,
  with the same automatic scope lifetime and locale switching as root bindings.
  Span refresh does not create a root `Text`; hierarchy and styles remain
  application-owned. Includes native BSN constructors and module release.

### Changed

- **Breaking:** `set_locale` keeps the active language, resources and text until
  every currently desired target leaf validates, then switches automatically at
  the publication boundary. Target failures leave active resources usable; the
  latest target wins, selecting the active language cancels the pending switch,
  and explicit retries avoid per-frame retry loops. `locale()` reports the active
  language; target inspection uses `prepared_locale`/`preparation_status`. See the
  [migration guide](docs/migration-0.3.md#let-language-changes-finish-automatically).
- **Breaking:** plugin and controller loading default to `Lazy` instead of
  `Full`. Inserted text bindings and localized required-resource systems request
  and retain only their scopes; without consumers, manual pins or leases, modules are
  not loaded.
  Select explicit `Full` for eager loading or direct world polling, or retain
  explicit `Manual` for manual requests. See the [migration guide](docs/migration-0.3.md).
- **Breaking:** the previous explicit-request `Lazy` mode is renamed to `Manual`,
  and `new_lazy` becomes `new_manual`. The new default `Lazy` loads scopes
  automatically from consumers; `Full` keeps its eager behavior.
- **Breaking:** default features select Bevy 0.20 instead of 0.19. Older
  applications disable defaults and select their backend explicitly.
- **Breaking:** the Rust minimum is 1.97.1, matching stable Bevy 0.20. Runtime
  0.2.2 is the last release supporting Rust 1.95.
- Compatibility checks and examples target stable Bevy 0.20. Bevy 0.16.1,
  0.17, 0.18 and 0.19 remain supported with the new compiler minimum.

### Fixed

- Explicit reloads wait for the normal watcher read started after a locale
  commit, preventing an older read from replacing a fresh translation.
- Releasing the last automatic consumer drops its target demand before queued
  acquisition failures are reported, avoiding rejection notifications for
  modules no longer requested.
- Native progress publication checks provider-wide preparation readiness once
  per changed publication instead of repeating a full scan for every scope.

### Removed

- Prerelease-only dependency setup; the final RC.1 and RC.2 snapshots remain
  available through their preview tags.

## [0.2.2] - 2026-10-06

Runtime and its build adapter require generator 0.2.2. See the
[migration from 0.2.1](docs/migration-0.2.2.md).

### Added

- Typed configuration and build errors re-exported from the generator by the
  manifest/build facade, preserving original error causes and filesystem context.

- Generated named embedded manifest constants, with relative schema selectors
  and ordinary catalog/resource aliases preserved.

### Changed

- **Breaking:** `from_cargo()` and `generate()` return `BuildError`; configuration
  parsing uses `ConfigError` and typed `ManifestError` variants. The standard
  `build() -> ExitCode` setup and runtime catalog signatures remain unchanged.

- **Breaking:** Cargo localization metadata uses a single package-relative
  `catalog` path; the generator no longer configures an engine asset root.
- **Breaking:** `texts::manifest()` retains generated `CATALOG_PATH` verbatim.
  Applications configure AssetServer roots or supply an explicit runtime origin.
- Unknown configuration fields are ignored while recognized fields are validated.
- Quickstarts, loading guides and runnable examples use the same file/embedded
  source contract and document engine roots separately from generation paths.

### Removed

- **Breaking:** generated `ASSET_ROOT` and `CATALOG_ASSET_PATH`;
  use application-owned roots and `CATALOG_PATH` for build filesystem metadata.
- **Breaking:** selector expression arguments to `embed_manifest!` and generated
  leaf-name helper macros; use the empty invocation or named constant declarations.

## [0.2.1] - 2026-09-29

Runtime 0.2.1 uses generator 0.2.1. Its Bevy build adapter is included in the
runtime crate; no separate bridge release is required.

### Added

- `LocalizationPlugin::from_bytes` accepts ready module buffers, and
  `from_loader` retrieves readable bytes asynchronously per requested leaf.
  Both support Full/Lazy, generated providers, native resources and readiness
  without a runtime manifest or AssetPlugin. See the [byte-source guide](GUIDE.md#custom-byte-sources).
- Backend-only configurations with defaults disabled support handwritten providers
  without the generator package. The [advanced guide](GUIDE.md#without-code-generation)
  and runnable examples cover generated byte input and the minimal provider.
- Generated `texts::manifest()` supplies the build-configured file-source
  contract without reading files or embedding FTL. Explicit runtime manifests
  and embedded selection remain available.
- `LocalizationAppExt::add_localized_systems` infers readiness from native,
  direct `Res<Scope>` parameters and waits without blocking the frame.
  Lazy loading and module lifetime remain controlled by explicit requests.
- `localized` applies the same readiness behavior before normal Bevy scheduling
  configuration; tuples wait independently.
- `add_localized_startup_systems` runs deferred initialization once after its
  catalogs are ready, preserving commands and ordinary error handling.
- [Optional adoption from 0.2.0](docs/migration-0.2.1.md), including startup,
  scheduling and parameter-shape limitations.

### Changed

- The build adapter and output-inclusion macros now live inside
  `bevy_fluent_typed`; standard `build()` / `translations!` calls are unchanged.
  Direct bridge users should move to the public facade.
- **Feature migration:** manifest constructors and types require `manifest`,
  enabled by default and by `codegen`. Handwritten providers using defaults
  disabled must add `manifest` to retain those APIs, or choose byte sources.
  See [migration details](docs/migration-0.2.1.md#byte-sources-and-optional-manifest-support).

### Fixed

- Readiness conditions register only the catalog resources they inspect, allowing
  systems that mutate unrelated resources or components to run in parallel.
- Idle frames reuse module demand and publication revisions instead of allocating
  temporary signatures and repeatedly scanning the entire schema.
- Text refresh visits scope types with live bindings. Unused scopes no longer
  add two scheduled systems each. Adding `Text` or `Text2d` to an existing binding
  formats it on the next refresh.
- File loading indexes exact asset addresses instead of scanning all known
  locale/module combinations. Embedded lookup uses the generator's 0.2.1 index.
- The asset-source guide's loading-boundary link also works in published Rustdoc.
- README and Rustdoc quickstarts use small, sequential setup blocks. Console
  runners, process exit and failure handling live in a separate loading recipe.

## [0.2.0] - 2026-09-28

This release updates runtime and bridge to 0.2.0. Follow the
[migration from 0.1.3](docs/migration-0.2.md) for the minimal Full-mode upgrade,
optional Lazy loading and handwritten providers.

### Added

- `Full` and `Lazy` modes with explicit root, group and leaf requests in Lazy.
- Typed navigation through partially loaded trees, scoped messages and
  independently available module resources.
- Explicit file/embedded `LocalizationManifest` sources. Embedded leaves and
  groups are selected by generated Rust paths, with `use` aliases supported.
  Unselected payloads stay out of unoptimized binaries.

### Changed

- **Breaking:** plugin constructors receive a parsed manifest contract rather
  than a manifest path string. The plugin no longer loads/watches that TOML.
- **Breaking:** startup and language changes require readiness handling, including
  with embedded sources; `catalog()` returns `Option<&C>`.
- **Breaking:** Full loads the selected language. Locale switches release previous
  runtime-owned language data while preserving logical module requests.
- **Breaking:** checked leaves publish independently; same-language failures retain
  the affected leaf's last good value. There is no whole-language transaction.
- Unavailable text bindings clear. Successful reloads publish fresh snapshots;
  idle frames and unchanged siblings preserve resource identity.

#### Bridge

- **Breaking:** generated and handwritten providers use `FluentScope`, per-leaf
  `Module` parsers and ready-child assembly instead of whole-catalog parsing and
  provider-owned resource publication.
- Runtime, bridge and required generator versions align at 0.2.0. The public
  build facade and host/runtime feature isolation are retained.

### Removed

- **Breaking:** implicit embedded startup/fallback and all-language preloading.
- **Breaking:** `CatalogDescriptor`, `ModuleSource`, provider `descriptor`,
  `embedded`, `parse` and `publish_resources` contracts, and `Module::embedded`.
  See the migration table for replacements.
- **Breaking, bridge:** `validate_definition`; use the shared manifest contract.

## [0.1.3] - 2026-09-22

### Changed

- Make the quick start, asset-root configuration, compatibility policy and
  comparison with bevy_fluent easier to find and follow.
- Clarify application-owned ICU services and locale-aware deferred formatting.
- Document and test named Bevy asset sources, pack layout, explicit reloads
  and last-good recovery. No archive parser or dependency is added.
- Add a codegen-enabled custom-source example and separate regression tests from
  runnable application code throughout the examples.
- Require bridge 0.1.3, whose build dependency requires generator 0.1.4 for updated
  generated documentation. Host/runtime dependency isolation is unchanged.

This documentation release changes no public runtime API or behavior.
Bridge 0.1.3 changes only dependency minimums and documentation.
Earlier releases predate this changelog; their source is retained in Git tags.

[0.4.0]: https://github.com/SDA-31/bevy_fluent_typed/compare/v0.3.1...release/0.4.0
[0.3.1]: https://github.com/SDA-31/bevy_fluent_typed/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/SDA-31/bevy_fluent_typed/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/SDA-31/bevy_fluent_typed/compare/v0.2.1...v0.2.2
[0.2.1]: https://docs.rs/crate/bevy_fluent_typed/0.2.1
[0.2.0]: https://github.com/SDA-31/bevy_fluent_typed/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/SDA-31/bevy_fluent_typed/compare/v0.1.2...v0.1.3
