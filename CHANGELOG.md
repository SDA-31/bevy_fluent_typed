# Changelog

Notable changes to the runtime and its companion bridge are recorded here using
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html);
before 1.0, a minor release can introduce incompatible API changes.

## [Unreleased]

### Added

- `LocalizationAppExt::add_localized_systems` infers readiness from native,
  direct `Res<Scope>` parameters and waits without blocking the frame.
  Lazy loading and module lifetime remain controlled by explicit requests.
- `localized` applies the same readiness behavior before normal Bevy scheduling
  configuration; tuples wait independently.
- `add_localized_startup_systems` runs deferred initialization once after its
  catalogs are ready, preserving commands and ordinary error handling.
- [Optional adoption from 0.2.0](docs/migration-0.2.1.md), including startup,
  scheduling and parameter-shape limitations. Existing 0.2.0 code remains valid.

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

[Unreleased]: https://github.com/SDA-31/bevy_fluent_typed/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/SDA-31/bevy_fluent_typed/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/SDA-31/bevy_fluent_typed/compare/v0.1.2...v0.1.3
