# Performance and measurement scope

Automatic bindings avoid unnecessary formatting and target writes, while
retaining metadata for catalog readiness, Msg identity and query eligibility.
This page describes the 0.4.0 contract and a measured implementation snapshot;
the [changelog](../CHANGELOG.md#040) records release status.

## Work skipped by automatic bindings

For existing, eligible built-in text targets, a ready unchanged catalog and the
default `CachePolicy::Rendered`:

| Refresh scenario | Formatter calls |
| --- | ---: |
| No relevant input or target change | 0 |
| Replace an existing binding with a clone of its current Msg | 0 |
| Replace one binding with a new Msg identity | 1 for that binding |
| Repair an ordinary application edit of the target text | 0 additional calls; reuse retained output |

Creating a new Msg establishes a new identity even if its closure returns the
same string. Cloning preserves identity; attaching that clone to another entity
creates an independent binding and result cache. Catalog publication invalidates
the selected scope, not individual Fluent keys. Direct `Msg::render` invokes
the formatter on every call and is outside this binding cache.

Built-in targets compare output before marking their component changed.
Custom adapters can use the same `LocalizationUpdate::set_text` helper, avoiding
redundant change notifications to downstream Bevy systems. Other application or
engine writes can still invalidate text and layout independently.

`CachePolicy::None` removes persistent formatted output from binding metadata.
Target components still own their strings. Built-ins skip unchanged targets;
repairs format again. Custom adapters decide when to read text and can inspect
`is_changed` and `component_changed` first. Repeated reads within an update and
its deferred continuation reuse the result. An apply-only adapter that always
reads text formats on every eligible refresh under None.

Small eligible updates run sequentially. Large updates can use Bevy's native
query batching and existing compute pool; structural callbacks remain sequential.
The pass still visits binding metadata, and parallel work completes before the
refresh returns. There is no startup cost calibration, frame budget or automatic
spreading across frames. See the [binding contract](bindings.md) for lifecycle,
filter reentry, bypass-change-detection and accessibility limits.

## Measured tradeoffs against runtime 0.3.1

These results compare published runtime 0.3.1 (`d08269d`) with implementation
snapshot `8a14049`, using the same benchmark scenarios.

The October 17, 2026 scale series used a Ryzen 5 4600H, Rust 1.99, Bevy 0.20,
release opt-level 3, Rendered caching and a pool of six workers. Three independent
processes per variant recorded 32 individual synchronous PostUpdate refreshes
each, after warmup. Variants were interleaved; no outliers were discarded.
Spawn, input preparation, renderer, layout, loading and validation were outside
the refresh timer. CPU was recorded in a separate series and sums all threads.

| Scenario | Stable wall p50, ms | Snapshot wall p50, ms | Stable / snapshot CPU mean, ms |
| --- | ---: | ---: | ---: |
| Refresh 100,000 texts through Fluent | 30.882 | 13.214 | 31.380 / 66.387 |
| Refresh 1,048,576 texts through Fluent | 329.135 | 111.877 | 329.173 / 535.195 |
| 100,000 unchanged texts | 0.309 | 0.644 | 0.346 / 0.700 |
| Refresh 100,000 texts with a String-clone formatter | 2.047 | 2.709 | 2.122 / 12.833 |

Wall p50 is the median of process medians; CPU mean is the median of process
means. Lower latency can consume more aggregate CPU, and cheap or unchanged
work can cost more than 0.3.1. Scheduling also depends on placement: a separate
256-refresh-per-process stress test with expensive callbacks clustered in the
last 5% of 100,000 entities measured 22.518 ms stable versus 45.126 ms with native
batching. These finite workloads establish neither a universal speedup nor a
worst-case bound for arbitrary callbacks.

Actual frame impact also includes text layout and other systems competing for
the same workers. Profile a representative application, including locale
switches and ordinary UI changes, before choosing a cache policy or tuning work.
