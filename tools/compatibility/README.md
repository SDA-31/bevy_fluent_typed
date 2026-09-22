# Exact Bevy compatibility checks

This maintainer-only Rust command creates disposable library/example workspaces,
pins the consumer's selected official Bevy packages to the requested release, then runs:

- Runtime tests, including named filesystem/in-memory asset sources, explicit
  reload and missing-module recovery, and actual text-detector ordering.
- Generated consumer tests, including multi-file filesystem watcher reloads.
- Both headless example binaries.
- Separate minimal codegen/no-codegen examples, including actual resource/Text
  values after switching through all three languages.
- The standalone ICU resource example: Decimal and percentage formatters,
  EN/ES/RU/AR UI/world strings, exact ratio scaling and explicit rebinding after
  formatter-policy changes. ICU is an example dependency, not a runtime feature.
- Explicit-build Cargo rebuild probes: edits, added/deleted modules and locales,
  missing build.rs, validation failures/recovery, excluded-asset tracking
  and repeated unchanged checks. After an edit, upstream's watched staging files
  may cause one extra timestamp-settling rebuild; two subsequent checks must be
  fresh. This is a bounded known limitation, not permission for a rebuild loop.
- Target dependency isolation with proc-macro edges excluded: the codegen feature
  must not link the build-time generator or formatting tools into the app.
- Build-only facade checks: no Bevy dependency or backend feature. Cargo artifacts
  from the real generated consumer must show separate `build`-only host and
  runtime feature sets, including on unchanged checks.
- A compile probe: generated `ResMut` is rejected on 0.19/0.20 and accepted by older ECS
  versions. The generated catalog API remains read-only on every backend.
- Runtime-only dependency isolation; on 0.19/0.20, rejected missing/conflicting backend selections.

No Python, shell-script runtime, game source or game assets are required. This
package is not a dependency of the library, bridge or example and is not published.
The `examples/asset_source` generated-resource example is additionally tested on
0.19 and the pinned 0.20 RC, including its loading/recovery tests and executable.
The runtime's virtual-source regression runs on every backend in this matrix.

This Git-only branch also accepts exactly `0.20.0-rc.1`, not arbitrary prereleases
or stable 0.20. Run it with Rust 1.96+; see
[preview policy and setup](../../docs/compat-bevy-0.20.md).

From the library repository, with a separate generator checkout:

```sh
cargo run --manifest-path tools/compatibility/Cargo.toml -- \
  --generator /absolute/path/to/fluent_typed_codegen \
  --target-dir /absolute/path/to/build-cache \
  0.16.1 0.17.0 0.18.0 0.19.0
```

In an enclosing workspace that lists this tool as a member:

```sh
cargo run -p bevy-fluent-compatibility -- \
  --generator crates/fluent_typed_codegen \
  --target-dir target/bevy-compatibility \
  0.16.1 0.17.0 0.18.0 0.19.0
```

The generator path is explicit; no sibling checkout convention is assumed.
The 0.16 backend starts at 0.16.1. The earlier 0.16.0 `bevy_color` package is
yanked and cannot be selected by a fresh exact-release fixture.
The 0.16.1 family pins `bevy_color` to 0.16.2 because that is the published
renderer's minimum; the other official packages remain pinned to 0.16.1.
Add `--offline` **after** `--` once the needed dependencies are cached. Each test
fixture has its own lockfile; the consuming workspace's lockfile is untouched.
Fixtures are copied once at the start of each release check. Re-run after edits.
The generated examples select their backend directly on the normal dependency
inside the fixture. They do not forward backend flags to the build-dependency.
Bevy pins live in a separate test-only package, leaving the tested library's
dependency requirements unchanged. Optional backend entries in a lockfile are
not evidence that those backends were compiled.
Release-family validation matches package names and versions together: metadata
from an inactive backend must not classify another version of an independently
released dependency as part of the selected engine release.

The tool prints each temporary fixture path and retains it, even on failure, for
inspection. `verified-metadata.json` records workspace-resolved packages,
`localization-example-dependencies.txt` and `bevy_fluent_typed-dependencies.txt`
record the selected normal/build dependency trees, and
`immutability-probe.log` records the expected compiler outcome. Remove these
specific temporary directories yourself when the audit is no longer needed.
The optional shared target directory reuses build artifacts across runs.
Completed fixtures contain an intentional `immutable_probe` compile-fail binary
on 0.19/0.20; create a fresh fixture through this command when repeating the suite.

[MIT](LICENSE), independently of any consuming application.
