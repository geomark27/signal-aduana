//! Board calculations: pure functions (no Tauri, no clock) ported from `ui/app.js`.
//! `today` always arrives as a parameter.

pub mod board;
pub mod calendar;
pub mod hours;
pub mod issue;
pub mod view;
pub mod week;

pub use view::{compute_view, View, ViewInput};
