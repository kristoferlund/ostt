//! Shared generic UI components for ostt.
//!
//! Contains reusable UI widgets and components that are used
//! by multiple features throughout the application.

pub mod components;
pub(crate) mod scroll;
pub(crate) mod session;
pub(crate) use components::footer::render_themed_footer;
pub(crate) use components::title::render_themed_title;

pub(crate) use keys::{cancel_requested, is_cancel_key};
mod keys;

pub use components::{
    render_app_layout, render_footer, render_title, render_toast, AppLayout, Toast,
};
