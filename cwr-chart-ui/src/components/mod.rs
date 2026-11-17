//! Reusable Dioxus RSX components for CWR chart apps.
//!
//! All styling lives in `assets/css/theme.css`; components only set class names.

mod chart_card;
mod chart_container;
mod date_range_picker;
mod error_display;
mod loading_spinner;
mod reservoir_selector;
mod snow_station_selector;
mod sort_selector;

pub use chart_card::{Callout, ChartCard};
pub use chart_container::ChartContainer;
pub use date_range_picker::DateRangePicker;
pub use error_display::ErrorDisplay;
pub use loading_spinner::LoadingSpinner;
pub use reservoir_selector::ReservoirSelector;
pub use snow_station_selector::SnowStationSelector;
pub use sort_selector::SortSelector;
