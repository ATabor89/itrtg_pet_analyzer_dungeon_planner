//! Prototype-only storage and file picking. Never reads or writes egui state.
use crate::app::Session;

#[cfg(not(target_arch = "wasm32"))]
fn state_path() -> Result<std::path::PathBuf, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(executable.with_file_name("slint-prototype-state.yaml"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load() -> Result<Option<Session>, String> {
    match std::fs::read_to_string(state_path()?) {
        Ok(text) => Session::from_yaml(&text).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save(session: &Session) -> Result<(), String> {
    let path = state_path()?;
    let temporary = path.with_extension("yaml.tmp");
    let text = serde_yaml::to_string(session).map_err(|e| e.to_string())?;
    std::fs::write(&temporary, text).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, &path).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "itrtg_slint_prototype_v1";

#[cfg(target_arch = "wasm32")]
fn storage() -> Result<web_sys::Storage, String> {
    web_sys::window().ok_or("Browser window unavailable")?
        .local_storage().map_err(|_| "Browser storage access denied")?
        .ok_or_else(|| "Browser storage unavailable".into())
}

#[cfg(target_arch = "wasm32")]
pub fn load() -> Result<Option<Session>, String> {
    storage()?.get_item(STORAGE_KEY).map_err(|_| "Cannot read browser storage")?
        .map(|text| Session::from_yaml(&text)).transpose()
}

#[cfg(target_arch = "wasm32")]
pub fn save(session: &Session) -> Result<(), String> {
    let text = serde_yaml::to_string(session).map_err(|e| e.to_string())?;
    storage()?.set_item(STORAGE_KEY, &text).map_err(|_| "Cannot save browser storage (disabled or full)".into())
}

/// Use the same callback on both platforms. Native I/O is off the UI thread;
/// the browser uses its file picker and an asynchronous read.
#[cfg(not(target_arch = "wasm32"))]
pub fn pick_file(ui: slint::Weak<crate::MainWindow>) {
    std::thread::spawn(move || {
        let result = rfd::FileDialog::new().add_filter("Pet Stats export", &["txt", "csv"])
            .pick_file().map(|path| std::fs::read_to_string(path).map_err(|e| e.to_string()));
        let _ = ui.upgrade_in_event_loop(move |ui| super::bindings::finish_file_pick(&ui, result));
    });
}

#[cfg(target_arch = "wasm32")]
pub fn pick_file(ui: slint::Weak<crate::MainWindow>) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = match rfd::AsyncFileDialog::new().add_filter("Pet Stats export", &["txt", "csv"]).pick_file().await {
            Some(file) => Some(String::from_utf8(file.read().await).map_err(|_| "Please choose a UTF-8 text export".into())),
            None => None,
        };
        if let Some(ui) = ui.upgrade() { super::bindings::finish_file_pick(&ui, result); }
    });
}

#[cfg(target_arch = "wasm32")]
pub fn fit_browser(ui: &crate::MainWindow) {
    use slint::ComponentHandle;
    use wasm_bindgen::{JsCast, closure::Closure};
    fn resize(ui: &crate::MainWindow) {
        if let Some(window) = web_sys::window() {
            let width = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1000.0);
            let height = window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(660.0);
            ui.window().set_size(slint::LogicalSize::new(width as f32, height as f32));
        }
    }
    resize(ui);
    let weak = ui.as_weak();
    let callback = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        if let Some(ui) = weak.upgrade() { resize(&ui); }
    });
    if let Some(window) = web_sys::window()
        && window.add_event_listener_with_callback("resize", callback.as_ref().unchecked_ref()).is_ok() {
        callback.forget(); // The one browser window owns this listener for its lifetime.
    }
}
