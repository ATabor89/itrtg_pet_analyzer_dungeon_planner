#![cfg_attr(not(target_arch = "wasm32"), windows_subsystem = "windows")]

slint::include_modules!();

mod app;
mod bindings;
mod platform;

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    bindings::wire(&ui).map_err(slint::PlatformError::Other)?;
    #[cfg(target_arch = "wasm32")]
    {
        ui.show()?;
        platform::fit_browser(&ui);
    }
    ui.run()
}
