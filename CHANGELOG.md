# Changelog

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
