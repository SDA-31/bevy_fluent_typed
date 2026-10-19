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

Parallel query execution requires Bevy's native `multi_threaded` feature.
A multi-worker compute pool alone does not enable parallel ECS queries.

## Measured refresh costs

These measurements use implementation snapshot
[`490eb5b`](https://github.com/SDA-31/bevy_fluent_typed/commit/490eb5bb1a457ee947abeb40c7e187a63caf1e05),
measured on October 19, 2026: Ryzen 5 4600H, Rust 1.99.0, exact Bevy 0.20.0,
release opt-level 3 and a six-worker compute pool with native multithreading.
Bindings share one catalog scope. The fixture uses a six-variant enum locale,
matching generated applications, and supplies already-ready catalogs directly.

| Text bindings | Scenario | Cache policy | Median elapsed time, ms | 95th-percentile elapsed time, ms | Mean aggregate CPU time, ms |
| ---: | --- | --- | ---: | ---: | ---: |
| 1,024 | Fluent formatting | Rendered | 0.373 | 0.390 | 0.385 |
| 100,000 | Unchanged targets | Rendered | 1.049 | 1.170 | 1.062 |
| 100,000 | String-clone formatter | Rendered | 2.948 | 5.035 | 14.508 |
| 100,000 | Fluent formatting | Rendered | 13.689 | 16.479 | 75.823 |
| 100,000 | Reattach the same Msg | Rendered | 1.094 | 1.227 | 1.097 |
| 100,000 | Replace one Msg | Rendered | 1.052 | 1.245 | 1.077 |
| 100,000 | Forced long-string growth | Rendered | 57.345 | 61.704 | 264.949 |
| 100,000 | Repair ordinary text edits | Rendered | 42.429 | 52.877 | 173.348 |
| 1,048,576 | Unchanged targets | Rendered | 11.450 | 12.095 | 11.193 |
| 1,048,576 | String-clone formatter | Rendered | 29.333 | 30.595 | 149.428 |
| 1,048,576 | Fluent formatting | Rendered | 123.241 | 147.506 | 621.995 |
| 100,000 | Unchanged targets | None | 1.040 | 1.200 | 1.065 |
| 100,000 | Fluent formatting | None | 11.194 | 11.769 | 51.576 |
| 100,000 | Forced long-string growth | None | 32.994 | 38.781 | 155.200 |
| 100,000 | Repair ordinary text edits | None | 61.561 | 66.700 | 246.293 |
| 100,000 | Unchanged targets across six fixed locale slots | Rendered | 1.093 | 1.217 | 1.121 |
| 100,000 | Fluent formatting across six fixed locale slots | Rendered | 11.470 | 12.465 | 59.548 |

For ten unchanged bindings, a separate block control measured 1.67 µs
elapsed and 1.67 µs aggregate CPU per refresh. This is the median of
five process means, each from fifteen blocks of 2,048 refreshes; batching
amortizes CPU-clock overhead for this small workload.

Elapsed time measures each synchronous `PostUpdate` refresh from start to
completion, including waiting for worker tasks. The table takes the median
of three process medians; the percentile column likewise reports the median
of three within-process percentiles. Each process records at least 64 refreshes
for the main workloads and 32 at one million targets. Fast workloads continue
toward a 150 ms observation window, with a 4,096-sample cap that can end it
earlier. Warmup is excluded; measured outliers are retained. These
are descriptive samples, not confidence intervals or worst-case bounds.

CPU is recorded in a separate trial. It sums all process threads during the
refresh interval, including the caller and workers. Each process contributes
its mean, and the table reports the median of those three means. CPU time is
not added to elapsed time; lower latency can require more aggregate CPU.

Spawn, input preparation, catalog loading/demand coordination, layout,
renderer and validation are outside the refresh timer. This is an isolated
binding-update benchmark, not a complete `App::update` or frame measurement.
The growth case deliberately clears and shrinks target buffers before each
refresh, forcing long outputs to allocate again; it is a stress workload.
The six-locale case republishes six ready snapshots with identical Fluent
payloads and distributes bindings across them; it measures selection and
refresh, not translation loading.

Actual frame impact also includes text layout and other systems competing
for the same workers. Profile a representative application, including
locale switches and ordinary UI changes, before tuning work or choosing
a cache policy. Callback placement and cost can change scheduling behavior.
