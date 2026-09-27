# Changelog

## Unreleased: explicit module loading

- Plugin constructors accept a shared immutable LocalizationManifest; generated payloads are opt-in.
- Full and Lazy modes select eager selected-language loading or explicit root/group/leaf demand; overlapping requests remain independent.
- Typed module navigation, partial resources and scoped messages support incomplete trees.
- Unavailable bindings clear, locale changes release old-language state, and same-language reloads publish checked leaves independently.
- Breaking provider migration: `FluentScope`, per-leaf `Module` parsers and ready-child
  assembly replace descriptor/whole-catalog parsing. See the
  [migration guide](GUIDE.md#migration-from-registry-013).
- Build/runtime feature isolation is retained; runtime uses only the generator package's manifest support.

## 0.1.3 — 2026-09-22

Documentation and examples refresh; no public runtime API or behavior changes.

- Make the quick start, asset-root configuration, compatibility policy and
  comparison with bevy_fluent easier to find and follow.
- Clarify application-owned ICU services and locale-aware deferred formatting.
- Document and test existing support for named Bevy asset sources, pack layout,
  explicit reloads and last-good recovery. No archive parser or dependency is added.
- Add a codegen-enabled custom-source example and separate regression tests from
  runnable application code throughout the examples.
- Require bridge 0.1.3, whose build dependency requires generator 0.1.4 for the
  updated generated documentation. Host/runtime dependency isolation is unchanged.

The companion `bevy_fluent_codegen_bridge` 0.1.3 has only dependency-minimum and
documentation updates; its generated adapter behavior and public API are unchanged.

Earlier releases predate this changelog; their source is retained in Git tags.
