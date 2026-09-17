# Exact Bevy compatibility checks

This maintainer-only Rust command creates disposable library/example workspaces,
pins the consumer's selected official Bevy packages to the requested release, then runs:

- Runtime tests, including named asset sources and actual text-detector ordering.
- Generated consumer tests, including multi-file filesystem watcher reloads.
- Both headless example binaries.
- A compile probe: generated `ResMut` is rejected on 0.19 and accepted by older ECS
  versions. The generated catalog API remains read-only on every backend.
- Runtime-only dependency isolation; on 0.19, rejected missing/conflicting backend selections.

No Python, shell-script runtime, game source or game assets are required. This
package is not a dependency of the library, bridge or example and is not published.

From the library repository, with a separate generator checkout:

```sh
cargo run --manifest-path tools/compatibility/Cargo.toml -- \
  --generator /absolute/path/to/fluent_typed_codegen \
  --target-dir /absolute/path/to/build-cache \
  0.17.0 0.18.0 0.19.0
```

In an enclosing workspace that lists this tool as a member:

```sh
cargo run -p bevy-fluent-compatibility -- \
  --generator crates/fluent_typed_codegen \
  --target-dir target/bevy-compatibility \
  0.17.0 0.18.0 0.19.0
```

The generator path is explicit; no sibling checkout convention is assumed.
Add `--offline` **after** `--` once the needed dependencies are cached. Each test
fixture has its own lockfile; the consuming workspace's lockfile is untouched.
Fixtures are copied once at the start of each release check. Re-run after edits.
Bevy pins live in a separate test-only package, leaving the tested library's
dependency requirements unchanged. Optional backend entries in a lockfile are
not evidence that those backends were compiled.

The tool prints each temporary fixture path and retains it, even on failure, for
inspection. `verified-metadata.json` records workspace-resolved packages,
`localization-example-dependencies.txt` and `bevy_fluent_typed-dependencies.txt`
record the selected normal/build dependency trees, and
`immutability-probe.log` records the expected compiler outcome. Remove these
specific temporary directories yourself when the audit is no longer needed.
The optional shared target directory reuses build artifacts across runs.
Completed fixtures contain an intentional `immutable_probe` compile-fail binary
on 0.19; create a fresh fixture through this command when repeating the suite.

[MIT](LICENSE), independently of any consuming application.
