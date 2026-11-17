//! Shared Dioxus components and D3.js bridge for CWR chart apps.
//!
//! This crate provides:
//! - `components`: reusable RSX components (themed card, selectors, containers)
//! - `js_bridge`: installs the D3 chart scripts + theme and queues render calls
//! - `state`: reactive `AppState` built from Dioxus signals
//! - `dates`: `YYYYMMDD` <-> `YYYY-MM-DD` helpers
//! - `series`: downsampling for long time series

pub mod components;
pub mod dates;
pub mod js_bridge;
pub mod series;
pub mod state;
