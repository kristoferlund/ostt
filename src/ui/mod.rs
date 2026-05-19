//! Shared generic UI components for ostt.
//!
//! Contains reusable UI widgets and components that are used
//! by multiple features throughout the application.

pub mod components;
pub mod error;

pub use components::{
    centered_fixed_rect, render_dialog, render_error_dialog, render_toast, DialogAction, Toast,
    ToastStyle,
};
pub use error::ErrorScreen;
