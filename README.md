# california-water-reservoir-tk

Tools and charts for California reservoir and snowpack data from
[CDEC](https://cdec.water.ca.gov/). The charts are embedded in the
[torrho.com water series](https://torrho.com/blog/cwr-part-1/).

## Layout

| Crate | What it is |
|---|---|
| `cwr-cdec` | Core types: reservoirs, snow stations, observations, water years |
| `cwr-data` | Per-station delta + brotli codec shared by the CLI and the charts |
| `cwr-db` | In-memory SQLite layer the chart apps query |
| `cwr-cmd`, `cwr-cli` | `cwr-cli`: fetches CDEC data and splits it into per-station files |
| `cwr-chart-ui` | Shared Dioxus components, theme (`assets/css/theme.css`) and D3 chart scripts (`assets/js/`) |
| `chart-*`, `table-*` | One small Dioxus (WASM) app per chart or table |

## Working locally

```sh
cargo test --workspace
cargo clippy --workspace --all-targets

# Build chart bundles into dist/<app>/ (needs dioxus-cli 0.7: `cargo binstall dioxus-cli`)
.github/scripts/build-wasm.sh chart-total-water chart-water-years
```

Charts render inside a `.cwr-card` that paints its own opaque surface, so they
stay readable on any host page. The palette is dark by default; put
`data-cwr-theme="light"` on an ancestor element to switch to the light one.
All colors are CSS custom properties in `cwr-chart-ui/assets/css/theme.css`.

## Daily data pipeline

`.github/workflows/build.yml` runs every day at 18:00 UTC:

1. `cwr-cli incremental-query` fetches new reservoir observations (12 parallel
   shards), resuming from the previous run's cached data, or from the committed
   `fixtures/observations.csv` when there is no cache.
2. `cwr-cli split-observations` writes per-station files and fails the run if
   fewer than 90% of reservoirs have data.
3. The chart apps are rebuilt (some embed data at compile time) and published,
   with the data files, as `ghcr.io/afbase/california-water-reservoir-tk:latest`.
4. A `cwr-updated` repository dispatch tells `afbase/afbase.github.io` to redeploy.

GitHub disables scheduled workflows in public repositories after 60 days
without activity. Two things guard against that: the workflow re-enables itself
on every scheduled run, and `afbase.github.io` runs a daily watchdog
(`cwr-refresh.yml`) that re-enables and, if needed, starts this workflow.
