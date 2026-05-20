//! Shared generic UI components for ostt.
//!
//! Contains reusable UI widgets and components that are used
//! by multiple features throughout the application.

pub mod components;

pub use components::{
    centered_fixed_rect, dialog_content_area, render_app_layout, render_dialog,
    render_dialog_content, render_error_dialog, render_footer, render_title, render_toast,
    AppLayout, DialogAction, ErrorScreen, Toast, ToastStyle,
};
