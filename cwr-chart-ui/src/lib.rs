//! Shared Dioxus components and D3.js bridge for CWR chart apps.
//!
//! This crate provides:
//! - `js_bridge`: Rust wrappers for D3.js chart functions via `js_sys::eval()`
//! - `state`: Reactive AppState with Dioxus Signals
//! - `components`: Reusable RSX components (selectors, containers, etc.)

pub mod js_bridge;
pub mod state;
pub mod components;
