# Performance and measurement scope

Automatic bindings avoid unnecessary formatting and target writes, while
retaining metadata for catalog readiness, Msg identity and query eligibility.
This page describes the 0.4.0 contract and measured refresh costs;
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
Catalog source versions are captured once per pass. When they are unchanged,
selection skips snapshot-version lookups for rows whose Msg, locale and query
eligibility remain current. Target change observation still runs; adapters that
request an update resolve the selected catalog and its version together.
Tracking uses compact optional snapshot identities and the existing shared
language-demand token, without another ECS component.
The pass still visits binding metadata, and parallel work completes before the
refresh returns. There is no startup cost calibration, frame budget or automatic
spreading across frames. See the [binding contract](bindings.md) for lifecycle,
filter reentry, bypass-change-detection and accessibility limits.

Parallel query execution requires Bevy's native `multi_threaded` feature.
A multi-worker compute pool alone does not enable parallel ECS queries.

## Measured refresh costs

These measurements describe the 0.4.0 implementation, measured on
October 20, 2026: Ryzen 5 4600H, Rust 1.99.0, exact Bevy 0.20.0,
release opt-level 3 and a six-worker compute pool with native multithreading.
Bindings share one catalog scope. The fixture uses a six-variant enum locale,
matching generated applications, and supplies already-ready catalogs directly.

| Text bindings | Scenario | Cache policy | Median elapsed time, ms | 95th-percentile elapsed time, ms | Mean aggregate CPU time, ms |
| ---: | --- | --- | ---: | ---: | ---: |
| 1,024 | Fluent formatting | Rendered | 0.368 | 0.402 | 0.393 |
| 100,000 | Unchanged targets | Rendered | 0.901 | 1.053 | 0.905 |
| 100,000 | String-clone formatter | Rendered | 2.944 | 4.559 | 13.531 |
| 100,000 | Fluent formatting | Rendered | 13.893 | 15.887 | 69.218 |
| 100,000 | Reattach the same Msg | Rendered | 0.867 | 1.004 | 0.890 |
| 100,000 | Replace one Msg | Rendered | 0.860 | 1.042 | 0.925 |
| 100,000 | Forced long-string growth | Rendered | 57.768 | 62.065 | 267.044 |
| 100,000 | Repair ordinary text edits | Rendered | 44.881 | 52.844 | 180.690 |
| 1,048,576 | Unchanged targets | Rendered | 9.169 | 9.983 | 9.247 |
| 1,048,576 | String-clone formatter | Rendered | 27.799 | 29.540 | 136.815 |
| 1,048,576 | Fluent formatting | Rendered | 113.222 | 117.853 | 642.726 |
| 100,000 | Unchanged targets | None | 0.876 | 1.083 | 0.887 |
| 100,000 | Fluent formatting | None | 10.025 | 11.182 | 49.996 |
| 100,000 | Forced long-string growth | None | 31.648 | 36.908 | 149.353 |
| 100,000 | Repair ordinary text edits | None | 61.577 | 69.235 | 250.987 |
| 100,000 | Unchanged targets across six fixed locale slots | Rendered | 0.871 | 0.999 | 0.877 |
| 100,000 | Fluent formatting across six fixed locale slots | Rendered | 11.030 | 12.053 | 62.642 |

For ten unchanged bindings, a separate block control measured 1.65 µs
elapsed and 1.65 µs aggregate CPU per refresh. This is the median of
five process means, each from fifteen blocks of 2,048 refreshes; batching
amortizes CPU-clock overhead for this small workload.

Elapsed time measures each synchronous `PostUpdate` refresh from start to
completion, including waiting for worker tasks. The table takes the median
of three process medians; the percentile column likewise reports the median
of three within-process percentiles. Each process records at least 64 refreshes
for the main workloads and 32 at one million targets. Fast workloads continue
toward a 150 ms observation window, with a 4,096-sample cap that can end it
earlier. Warmup and output validation are outside the timer; measured outliers
are retained. These are descriptive samples, not confidence intervals or
worst-case bounds.

CPU is recorded in a separate trial. It sums all process threads during the
refresh interval, including the caller and workers. Each process contributes
its mean, and the table reports the median of those three means. CPU time is
not added to elapsed time; lower latency can require more aggregate CPU.
CPU-clock overhead is retained in the samples.

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
