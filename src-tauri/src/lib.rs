pub mod adapter;
pub mod compatibility;
pub mod desktop_config;
pub mod handoff;
pub mod models;
#[allow(dead_code)]
mod patch;
#[allow(dead_code)]
mod provider_fields;
pub mod restore;
pub mod storage;
pub mod vscode_config;

#[cfg(feature = "desktop")]
mod desktop {
    use super::*;
    use serde::Serialize;
    use std::sync::Mutex;
    use tauri::{Emitter, Manager};
    use tauri_plugin_deep_link::DeepLinkExt;

    struct AppState {
        paths: storage::Paths,
        busy: Mutex<bool>,
        discovery: Mutex<Option<models::DiscoverySession>>,
        handoff: Mutex<handoff::HandoffState>,
    }
    struct BusyGuard<'a>(&'a Mutex<bool>);
    impl<'a> BusyGuard<'a> {
        fn begin(lock: &'a Mutex<bool>) -> Result<Self, String> {
            let mut busy = lock
                .lock()
                .map_err(|_| "The configurator state is abnormal; please reopen it.")?;
            if *busy {
                return Err("Another operation is being processed; please wait.".into());
            }
            *busy = true;
            Ok(Self(lock))
        }
    }
    impl Drop for BusyGuard<'_> {
        fn drop(&mut self) {
            if let Ok(mut busy) = self.0.lock() {
                *busy = false;
            }
        }
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ClientStatus {
        client: models::Client,
        has_backup: bool,
        pending: bool,
        config_exists: bool,
        config_path: String,
        compatibility: compatibility::Compatibility,
    }
    #[derive(Serialize)]
    struct Status {
        sandbox: bool,
        clients: Vec<ClientStatus>,
    }

    #[tauri::command]
    async fn app_status(state: tauri::State<'_, AppState>) -> Result<Status, String> {
        let paths = state.paths.clone();
        tauri::async_runtime::spawn_blocking(move || Status {
            sandbox: paths.sandbox,
            clients: models::Client::ALL
                .into_iter()
                .map(|client| {
                    let path = paths.target(storage::FileKind::primary(client));
                    ClientStatus {
                        client,
                        has_backup: paths.has_backup(client),
                        pending: paths.pending(client),
                        config_exists: path.is_file(),
                        config_path: path.to_string_lossy().into_owned(),
                        compatibility: compatibility::inspect(&paths, client),
                    }
                })
                .collect(),
        })
        .await
        .map_err(|_| "Could not check the client status.".into())
    }
    #[tauri::command]
    async fn discover_models(
        state: tauri::State<'_, AppState>,
        input: models::DiscoveryInput,
    ) -> Result<models::DiscoveryResponse, String> {
        let _busy = BusyGuard::begin(&state.busy)?;
        *state
            .discovery
            .lock()
            .map_err(|_| "The model list state is abnormal.")? = None;
        let session = models::DiscoverySession::new(models::discover(input).await?);
        let response = session.response();
        *state
            .discovery
            .lock()
            .map_err(|_| "The model list state is abnormal.")? = Some(session);
        Ok(response)
    }
    #[tauri::command]
    fn import_config(
        state: tauri::State<'_, AppState>,
        input: models::SelectionInput,
    ) -> Result<adapter::ImportResult, String> {
        let _busy = BusyGuard::begin(&state.busy)?;
        let validated = state
            .discovery
            .lock()
            .map_err(|_| "The model list state is abnormal.")?
            .as_ref()
            .ok_or("Fetch the available models first.")?
            .prepare(&input)?;
        let mut result = adapter::apply(&state.paths, validated)?;
        result.provider_name = state
            .handoff
            .lock()
            .map_err(|_| "Could not read the imported gateway.")?
            .completion_provider(&result.base_url);
        state
            .handoff
            .lock()
            .map_err(|_| "Could not clear the platform import.")?
            .clear_accepted();
        *state
            .discovery
            .lock()
            .map_err(|_| "The model list state is abnormal.")? = None;
        Ok(result)
    }
    #[tauri::command]
    fn restore_config(
        state: tauri::State<'_, AppState>,
        client: models::Client,
        review_id: String,
        overwrite_modified: bool,
    ) -> Result<restore::RestoreResult, String> {
        let _busy = BusyGuard::begin(&state.busy)?;
        let result = restore::apply(&state.paths, client, &review_id, overwrite_modified)?;
        state
            .handoff
            .lock()
            .map_err(|_| "Could not clear the platform import.")?
            .clear_accepted();
        *state
            .discovery
            .lock()
            .map_err(|_| "The model list state is abnormal.")? = None;
        Ok(result)
    }
    #[tauri::command]
    fn restore_points(state: tauri::State<'_, AppState>) -> Vec<restore::RestorePoint> {
        restore::list(&state.paths)
    }
    #[tauri::command]
    fn pending_handoff(
        state: tauri::State<'_, AppState>,
    ) -> Result<Option<handoff::PendingHandoff>, String> {
        Ok(state
            .handoff
            .lock()
            .map_err(|_| "Could not read the platform import.")?
            .pending())
    }
    #[tauri::command]
    async fn accept_handoff(
        state: tauri::State<'_, AppState>,
        request_id: String,
    ) -> Result<handoff::SetupView, String> {
        let _busy = BusyGuard::begin(&state.busy)?;
        let link = {
            state
                .handoff
                .lock()
                .map_err(|_| "Could not read the platform import.")?
                .link(&request_id)?
        };
        let setup = handoff::redeem(&link, state.paths.sandbox && cfg!(debug_assertions)).await?;
        let view = state
            .handoff
            .lock()
            .map_err(|_| "Could not prepare the platform import.")?
            .accept(&request_id, setup)?;
        *state
            .discovery
            .lock()
            .map_err(|_| "Could not clear the previous model list.")? = None;
        Ok(view)
    }
    #[tauri::command]
    async fn discover_handoff_models(
        state: tauri::State<'_, AppState>,
        request_id: String,
        client: models::Client,
    ) -> Result<models::DiscoveryResponse, String> {
        let input = {
            state
                .handoff
                .lock()
                .map_err(|_| "Could not read the platform import.")?
                .discovery_input(&request_id, client)?
        };
        discover_models(state, input).await
    }
    #[tauri::command]
    fn cancel_handoff(state: tauri::State<'_, AppState>, request_id: String) -> Result<(), String> {
        let _busy = BusyGuard::begin(&state.busy)?;
        let cleared = state
            .handoff
            .lock()
            .map_err(|_| "Could not cancel the platform import.")?
            .cancel(&request_id);
        if cleared {
            *state
                .discovery
                .lock()
                .map_err(|_| "Could not clear the previous model list.")? = None;
        }
        Ok(())
    }
    fn receive_handoffs(app: &tauri::AppHandle, urls: &[reqwest::Url]) {
        let state = app.state::<AppState>();
        let mut changed = false;
        if let Ok(mut pending) = state.handoff.lock() {
            for url in urls {
                changed |=
                    pending.receive(url.as_str(), state.paths.sandbox && cfg!(debug_assertions));
            }
        }
        if changed {
            let _ = app.emit("connect-handoff", ());
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    }

    fn startup_error(message: &str) -> ! {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
            let title: Vec<u16> = "TokenToAPI Connect\0".encode_utf16().collect();
            let body: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
            // Both buffers are NUL-terminated and remain alive during the synchronous dialog.
            unsafe {
                MessageBoxW(
                    std::ptr::null_mut(),
                    body.as_ptr(),
                    title.as_ptr(),
                    MB_OK | MB_ICONERROR,
                );
            }
        }
        eprintln!("{message}");
        std::process::exit(1)
    }

    pub fn run() {
        #[cfg(windows)]
        if tauri::webview_version().is_err() {
            startup_error("Microsoft Edge WebView2 Runtime is missing on this machine, so the window cannot open yet.\n\nInstall WebView2 using the link in the bundled README, then reopen TokenToAPI Connect. The configurator itself needs no installation.");
        }
        let paths = storage::Paths::discover().unwrap_or_else(|error| startup_error(&error));
        if let Err(error) = tauri::Builder::default()
            .plugin(tauri_plugin_single_instance::init(|app, _, _| {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }))
            .plugin(tauri_plugin_deep_link::init())
            .manage(AppState {
                paths,
                busy: Mutex::new(false),
                discovery: Mutex::new(None),
                handoff: Mutex::new(handoff::HandoffState::default()),
            })
            .setup(|app| {
                #[cfg(any(windows, target_os = "linux"))]
                app.deep_link().register_all()?;
                let handle = app.handle().clone();
                app.deep_link()
                    .on_open_url(move |event| receive_handoffs(&handle, &event.urls()));
                if let Some(urls) = app.deep_link().get_current()? {
                    receive_handoffs(app.handle(), &urls);
                }
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
                pending_handoff,
                accept_handoff,
                discover_handoff_models,
                cancel_handoff,
                app_status,
                discover_models,
                import_config,
                restore_config,
                restore_points
            ])
            .run(tauri::generate_context!())
        {
            startup_error(&format!("Could not start TokenToAPI Connect: {error}"));
        }
    }
}
#[cfg(feature = "desktop")]
pub use desktop::run;
