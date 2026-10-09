//! Shared generic UI components for ostt.
//!
//! Contains reusable UI widgets and components that are used
//! by multiple features throughout the application.

pub mod components;
pub(crate) mod session;

pub(crate) use keys::{cancel_requested, is_cancel_key};
mod keys;

pub use components::render_app_layout;
