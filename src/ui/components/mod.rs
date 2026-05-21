pub mod app_layout;
pub mod dialog;
pub mod error_dialog;
pub mod error_screen;
pub mod footer;
pub mod title;
pub mod toast;

pub use app_layout::{render_app_layout, AppLayout};
pub use dialog::{
    centered_fixed_rect, dialog_content_area, render_dialog, render_dialog_content, DialogAction,
};
pub use error_dialog::render_error_dialog;
pub use error_screen::ErrorScreen;
pub use footer::render_footer;
pub use title::render_title;
pub use toast::{render_toast, Toast, ToastStyle};
