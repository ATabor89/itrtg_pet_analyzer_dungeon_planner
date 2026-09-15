//! Prototype-only storage and file picking. Never reads or writes egui state.
use crate::app::Session;

pub fn open_wiki(url: &str) -> Result<(), String> {
    if !url.starts_with("https://itrtg.wiki.gg/wiki/") { return Err("This is not a supported ITRTG wiki link.".into()); }
    #[cfg(not(target_arch = "wasm32"))]
    { webbrowser::open(url).map_err(|e| e.to_string()) }
    #[cfg(target_arch = "wasm32")]
    { web_sys::window().ok_or("Browser window unavailable")?.open_with_url_and_target(url, "_blank")
        .map_err(|_| "Could not open wiki page".to_string())?.ok_or("Browser blocked the wiki window")?; Ok(()) }
}

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
        let result = rfd::FileDialog::new().add_filter("Game export or full save", &["txt", "csv", "html", "htm"])
            .pick_file().map(|path| std::fs::read_to_string(path).map_err(|e| e.to_string()));
        let _ = ui.upgrade_in_event_loop(move |ui| super::bindings::finish_file_pick(&ui, result));
    });
}

#[cfg(target_arch = "wasm32")]
pub fn pick_file(ui: slint::Weak<crate::MainWindow>) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = match rfd::AsyncFileDialog::new().add_filter("Game export or full save", &["txt", "csv", "html", "htm"]).pick_file().await {
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


/// Native work runs on a worker; result callbacks stay on the UI thread.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_background<T: Send + 'static>(work: impl FnOnce() -> Result<T,String> + Send + 'static,
    done: impl FnOnce(Result<T,String>) + 'static) {
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || { let _ = send.send(work()); });
    poll_result(receive, done);
}
#[cfg(not(target_arch = "wasm32"))]
fn poll_result<T: Send + 'static>(receive: std::sync::mpsc::Receiver<Result<T,String>>,
    done: impl FnOnce(Result<T,String>) + 'static) {
    slint::Timer::single_shot(std::time::Duration::from_millis(16), move || {
        match receive.try_recv() {
            Ok(result) => done(result),
            Err(std::sync::mpsc::TryRecvError::Empty) => poll_result(receive, done),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => done(Err("Import stopped. Existing data kept.".into())),
        }
    });
}
/// Browser work is deferred for busy-state rendering; web workers remain future work.
#[cfg(target_arch = "wasm32")]
pub fn run_background<T: 'static>(work: impl FnOnce() -> Result<T,String> + 'static,
    done: impl FnOnce(Result<T,String>) + 'static) {
    slint::Timer::single_shot(std::time::Duration::from_millis(16), move || done(work()));
}
pub fn decode_save(text: String, done: impl FnOnce(Result<crate::app::PreparedSave,String>) + 'static) {
    run_background(move || crate::app::prepare_save(&text), done);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn install_drop(ui: &crate::MainWindow) {
    use slint::{ComponentHandle, winit_030::{WinitWindowAccessor, EventResult, winit}};
    let weak=ui.as_weak();
    ui.window().on_winit_window_event(move |_,event| {
        if let winit::event::WindowEvent::DroppedFile(path)=event {
            if let Some(ui)=weak.upgrade() && !ui.get_import_busy() && !ui.get_modal_open() {
                ui.set_import_busy(true);
                let path=path.clone(); let weak=ui.as_weak();
                run_background(move || std::fs::read_to_string(path).map_err(|e|e.to_string()), move |result| {
                    if let Some(ui)=weak.upgrade() { super::bindings::finish_drop(&ui,result); }
                });
            }
            EventResult::PreventDefault
        } else { EventResult::Propagate }
    });
}

#[cfg(target_arch = "wasm32")]
pub fn install_drop(ui: &crate::MainWindow) {
    use slint::ComponentHandle;
    use wasm_bindgen::{JsCast,closure::Closure};
    let Some(window)=web_sys::window() else { return; };
    let over=Closure::<dyn FnMut(web_sys::DragEvent)>::new(|event:web_sys::DragEvent|event.prevent_default());
    if window.add_event_listener_with_callback("dragover",over.as_ref().unchecked_ref()).is_ok() { over.forget(); }
    let weak=ui.as_weak();
    let drop=Closure::<dyn FnMut(web_sys::DragEvent)>::new(move |event:web_sys::DragEvent| {
        event.prevent_default();
        let Some(ui)=weak.upgrade() else { return; };
        if ui.get_import_busy() || ui.get_modal_open() { return; }
        let Some(file)=event.data_transfer().and_then(|d|d.files()).and_then(|files|files.get(0)) else { return; };
        ui.set_import_busy(true);
        let weak=ui.as_weak();
        wasm_bindgen_futures::spawn_local(async move {
            let result=wasm_bindgen_futures::JsFuture::from(file.text()).await
                .map_err(|_|"Could not read dropped text file".to_string())
                .and_then(|v|v.as_string().ok_or_else(||"Please drop a text export or HTML log".into()));
            if let Some(ui)=weak.upgrade() { super::bindings::finish_drop(&ui,result); }
        });
    });
    if window.add_event_listener_with_callback("drop",drop.as_ref().unchecked_ref()).is_ok() { drop.forget(); }
}
