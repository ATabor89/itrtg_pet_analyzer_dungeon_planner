//! Everything that differs between the desktop and browser builds: session
//! storage, the clipboard, file pickers/downloads, drag-and-drop and background
//! work. The egui app's own state file is only ever read (for a first-run
//! migration), never written.
use crate::app::Session;

/// A text file the user picked or dropped.
pub struct PickedFile { pub name: String, pub text: String }

pub const NATIVE: bool = cfg!(not(target_arch = "wasm32"));

/// Read the system clipboard's text. In the browser this asks for permission
/// the first time and resolves asynchronously.
pub fn read_clipboard(done: impl FnOnce(Result<String, String>) + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    { done(arboard::Clipboard::new().and_then(|mut c| c.get_text()).map_err(|e| format!("Could not read the clipboard: {e}"))); }
    #[cfg(target_arch = "wasm32")]
    {
        let Some(window) = web_sys::window() else { done(Err("Browser window unavailable".into())); return; };
        let promise = window.navigator().clipboard().read_text();
        wasm_bindgen_futures::spawn_local(async move {
            done(wasm_bindgen_futures::JsFuture::from(promise).await
                .map_err(|_| "The browser blocked clipboard access. Allow it for this page, or use Import data… and paste.".to_string())
                .and_then(|v| v.as_string().ok_or_else(|| "The clipboard does not hold text.".to_string())));
        });
    }
}

/// The egui app's saved settings, if any (first-run migration source).
pub fn load_legacy_state() -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    { std::fs::read_to_string("data/app_state.yaml").ok() }
    #[cfg(target_arch = "wasm32")]
    { storage().ok()?.get_item("app_state").ok().flatten() }
}

/// Fetch and parse the wiki's pet table (desktop only — the wiki sends no CORS
/// headers, so a browser cannot read it).
#[cfg(not(target_arch = "wasm32"))]
pub fn fetch_wiki() -> Result<Vec<itrtg_models::WikiPet>, String> {
    let client = reqwest::blocking::Client::builder().user_agent("pet_extractor/0.1.0 (ITRTG tool)").build().map_err(|e| e.to_string())?;
    let response = client.get("https://itrtg.wiki.gg/wiki/Pets?action=raw").send().map_err(|e| e.to_string())?;
    if !response.status().is_success() { return Err(format!("HTTP {}", response.status())); }
    let source = response.text().map_err(|e| e.to_string())?;
    wiki_extractor::parser::parse_pets(&source).map_err(|e| e.to_string())
}
#[cfg(target_arch = "wasm32")]
pub fn fetch_wiki() -> Result<Vec<itrtg_models::WikiPet>, String> {
    Err("Wiki refresh is not available in the browser. Drop or paste the wiki page source instead.".into())
}

/// Keep a refreshed wiki table for the next launch when running from the repo
/// (where `data/wiki_pets.yaml` exists); elsewhere it stays in memory.
pub fn save_wiki(pets: &[itrtg_models::WikiPet]) {
    #[cfg(not(target_arch = "wasm32"))]
    if std::path::Path::new("data/wiki_pets.yaml").exists() && let Ok(yaml) = serde_yaml::to_string(pets) {
        let _ = std::fs::write("data/wiki_pets.yaml", yaml);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = pets;
}

/// Let the user save `contents` as a file: a save dialog on the desktop, a
/// download in the browser. `done` gets `Ok(false)` when the dialog was cancelled.
pub fn save_text_file(name: String, contents: String, done: impl FnOnce(Result<bool, String>) + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    run_background(move || {
        let Some(path) = rfd::FileDialog::new().set_file_name(&name).add_filter("Save file", &["txt"]).save_file() else { return Ok(false) };
        std::fs::write(path, contents).map(|_| true).map_err(|e| e.to_string())
    }, done);
    #[cfg(target_arch = "wasm32")]
    done(download_text(&name, &contents).map(|_| true));
}

/// Trigger a browser download: Blob → object URL → a temporary `<a download>`.
#[cfg(target_arch = "wasm32")]
fn download_text(name: &str, contents: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    let document = web_sys::window().and_then(|w| w.document()).ok_or("Browser document unavailable")?;
    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(contents));
    let blob = web_sys::Blob::new_with_str_sequence(&parts).map_err(|_| "Could not prepare the download")?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(|_| "Could not prepare the download")?;
    let anchor = document.create_element("a").map_err(|_| "Could not prepare the download")?
        .dyn_into::<web_sys::HtmlAnchorElement>().map_err(|_| "Could not prepare the download")?;
    anchor.set_href(&url);
    anchor.set_download(name);
    // `click()` dispatches synchronously, so the download has begun before the URL is revoked.
    anchor.click();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}

pub fn copy_text(text:String,done:impl FnOnce(Result<(),String>)+'static) {
    #[cfg(not(target_arch="wasm32"))]
    { done(arboard::Clipboard::new().and_then(|mut c|c.set_text(text)).map_err(|e|e.to_string())); }
    #[cfg(target_arch="wasm32")]
    {
        let Some(window)=web_sys::window() else {done(Err("Browser window unavailable".into()));return;};
        let promise=window.navigator().clipboard().write_text(&text);
        wasm_bindgen_futures::spawn_local(async move {done(wasm_bindgen_futures::JsFuture::from(promise).await.map(|_|()).map_err(|_|"Clipboard permission denied".into()));});
    }
}

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
pub fn pick_file(done: impl FnOnce(Option<Result<PickedFile, String>>) + 'static) {
    run_background(|| Ok(rfd::FileDialog::new().add_filter("Game export, save or log", &["txt", "csv", "html", "htm"]).add_filter("All files", &["*"])
        .pick_file().map(|path| std::fs::read_to_string(&path).map_err(|e| e.to_string())
            .map(|text| PickedFile { name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), text }))),
        |result| done(result.unwrap_or(None)));
}

#[cfg(target_arch = "wasm32")]
pub fn pick_file(done: impl FnOnce(Option<Result<PickedFile, String>>) + 'static) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = match rfd::AsyncFileDialog::new().add_filter("Game export, save or log", &["txt", "csv", "html", "htm"]).pick_file().await {
            Some(file) => { let name = file.file_name(); Some(String::from_utf8(file.read().await).map(|text| PickedFile { name, text }).map_err(|_| "Please choose a UTF-8 text file".into())) },
            None => None,
        };
        done(result);
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
                run_background(move || std::fs::read_to_string(&path).map_err(|e|e.to_string())
                    .map(|text| PickedFile { name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), text }), move |result| {
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
        let name=file.name();
        wasm_bindgen_futures::spawn_local(async move {
            let result=wasm_bindgen_futures::JsFuture::from(file.text()).await
                .map_err(|_|"Could not read dropped text file".to_string())
                .and_then(|v|v.as_string().ok_or_else(||"Please drop a text export or HTML log".into()))
                .map(|text| PickedFile { name, text });
            if let Some(ui)=weak.upgrade() { super::bindings::finish_drop(&ui,result); }
        });
    });
    if window.add_event_listener_with_callback("drop",drop.as_ref().unchecked_ref()).is_ok() { drop.forget(); }
}
