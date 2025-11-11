pub mod app;
pub mod assets;
pub mod config;
pub mod settings;
pub mod ui;

pub use app::{render_ui, PatinaEguiApp};
pub use assets::{logo_color_image, logo_dimensions, logo_png_bytes};
pub use config::{
    load_provider_config, load_ui_settings, save_ui_settings, ProviderConfig, Scope, UiSettings,
};
