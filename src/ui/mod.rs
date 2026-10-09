//! Shared generic UI components for ostt.
//!
//! Contains reusable UI widgets and components that are used
//! by multiple features throughout the application.

pub mod components;
pub(crate) mod scroll;

pub(crate) use keys::{cancel_requested, is_cancel_key, is_ctrl_c};
mod keys;

pub use components::{
    render_app_layout, render_footer, render_title, render_toast, AppLayout, Toast,
};
