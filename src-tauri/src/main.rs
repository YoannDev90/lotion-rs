use lotion_rs::policy::PolicyManager;
use lotion_rs::security::SecurityModule;
use lotion_rs::theming::ThemeManager;

use lotion_rs::config::LotionConfig;
use lotion_rs::i18n::I18nManager;
use lotion_rs::spellcheck::SpellcheckManager;
use lotion_rs::state::AppState;
use rand::Rng;
use std::fs::{self, File};
use std::io::{Read, Write};
#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt; // Specific import for unix permissions
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;

pub static NEEDS_SAVE: AtomicBool = AtomicBool::new(false);

const SECRET_FILE_NAME: &str = "secret_key";

// Helper function to get or create the application secret
fn get_or_create_app_secret() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let secret_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("lotion-rs");
    let secret_path = secret_dir.join(SECRET_FILE_NAME);

    if secret_path.exists() {
        let mut file = File::open(&secret_path)?;
        let mut secret = vec![0u8; 32];
        file.read_exact(&mut secret)?;
        tracing::info!("Application secret loaded from {}", secret_path.display());
        Ok(secret)
    } else {
        tracing::info!(
            "Generating new application secret at {}",
            secret_path.display()
        );
        fs::create_dir_all(&secret_dir)?;

        let mut secret = vec![0u8; 32];
        rand::rng().fill_bytes(&mut secret);

        let mut file = File::create(&secret_path)?;
        #[cfg(target_family = "unix")]
        {
            // Set permissions to 0o600 (read/write only for owner)
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(&secret)?;
        Ok(secret)
    }
}

// Helper function to check if the command invocation origin is trusted
fn is_trusted_origin<R: tauri::Runtime>(
    webview: &tauri::Webview<R>,
    config: &LotionConfig,
) -> bool {
    if let Ok(url) = webview.url() {
        let origin = format!("{}://{}", url.scheme(), url.host_str().unwrap_or_default());
        let is_trusted = config.trusted_domains.iter().any(|o| origin == *o);

        if !is_trusted {
            // Development exception for localhost if explicitly enabled could go here
            #[cfg(debug_assertions)]
            if origin == "tauri://localhost" || origin == "wry://localhost" {
                return true;
            }

            tracing::warn!(
                "SECURITY ALERT: Untrusted origin '{}' attempted to invoke a privileged command.",
                origin
            );
        }
        is_trusted
    } else {
        tracing::error!("SECURITY ERROR: Could not determine origin for command invocation.");
        false
    }
}

#[tauri::command]
async fn get_window_tabs(
    webview: tauri::Webview<tauri::Wry>,
    window_id: String,
    state: tauri::State<'_, Arc<tokio::sync::Mutex<AppState>>>,
    config: tauri::State<'_, LotionConfig>,
) -> Result<Vec<lotion_rs::state::TabState>, String> {
    if !is_trusted_origin(&webview, &config) {
        return Err("Untrusted origin".into());
    }
    tracing::info!("get_window_tabs called from origin: {:?}", webview.url());
    let app_state = state.lock().await;
    if let Some(w_state) = app_state.windows.get(&window_id) {
        Ok(w_state
            .tab_ids
            .iter()
            .filter_map(|id| app_state.tabs.get(id))
            .cloned()
            .collect())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn switch_tab(
    webview: tauri::Webview<tauri::Wry>,
    tab_id: String,
    orchestrator: tauri::State<'_, Arc<dyn lotion_rs::traits::TabOrchestrator<tauri::Wry>>>,
    config: tauri::State<'_, LotionConfig>,
) -> Result<(), String> {
    if !is_trusted_origin(&webview, &config) {
        return Err("Untrusted origin".into());
    }
    orchestrator.show_tab(&tab_id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn close_tab(
    webview: tauri::Webview<tauri::Wry>,
    tab_id: String,
    _app: tauri::AppHandle<tauri::Wry>,
    orchestrator: tauri::State<'_, Arc<dyn lotion_rs::traits::TabOrchestrator<tauri::Wry>>>,
    state: tauri::State<'_, Arc<tokio::sync::Mutex<AppState>>>,
    app_secret_state: tauri::State<'_, Arc<Vec<u8>>>,
    config: tauri::State<'_, LotionConfig>,
) -> Result<(), String> {
    if !is_trusted_origin(&webview, &config) {
        return Err("Untrusted origin".into());
    }
    orchestrator
        .destroy_tab(&tab_id)
        .map_err(|e| e.to_string())?;

    let mut app_state = state.lock().await;
    app_state.tabs.remove(&tab_id);
    for window_state in app_state.windows.values_mut() {
        window_state.tab_ids.retain(|id| id != &tab_id);
        if window_state.active_tab_id.as_ref() == Some(&tab_id) {
            window_state.active_tab_id = window_state.tab_ids.last().cloned();
            if let Some(ref next_id) = window_state.active_tab_id {
                let _ = orchestrator.show_tab(next_id);
            }
        }
    }
    app_state
        .save_to_disk(app_secret_state.inner().as_slice())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn new_tab(
    webview: tauri::Webview<tauri::Wry>,
    window_id: String,
    app: tauri::AppHandle<tauri::Wry>,
    orchestrator: tauri::State<'_, Arc<dyn lotion_rs::traits::TabOrchestrator<tauri::Wry>>>,
    state: tauri::State<'_, Arc<tokio::sync::Mutex<AppState>>>,
    app_secret_state: tauri::State<'_, Arc<Vec<u8>>>,
    config: tauri::State<'_, LotionConfig>,
) -> Result<(), String> {
    if !is_trusted_origin(&webview, &config) {
        return Err("Untrusted origin".into());
    }
    let notion_url = "https://www.notion.so";
    let new_id = orchestrator
        .create_tab(&app, &window_id, notion_url)
        .map_err(|e| e.to_string())?;
    orchestrator.show_tab(&new_id).map_err(|e| e.to_string())?;

    let mut app_state = state.lock().await;
    if let Some(w_state) = app_state.windows.get_mut(&window_id) {
        w_state.tab_ids.push(new_id);
        app_state
            .save_to_disk(app_secret_state.inner().as_slice())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn update_tab_state(
    webview: tauri::Webview<tauri::Wry>,
    tab_id: String,
    title: String,
    url: String,
    state: tauri::State<'_, Arc<tokio::sync::Mutex<AppState>>>,
    _app_secret_state: tauri::State<'_, Arc<Vec<u8>>>,
    config: tauri::State<'_, LotionConfig>,
) -> Result<(), String> {
    if !is_trusted_origin(&webview, &config) {
        return Err("Untrusted origin".into());
    }

    if let Ok(webview_url) = webview.url() {
        if webview_url.as_str() != url {
            tracing::warn!("Origin {} mismatched URL", webview_url);
            return Err("Mismatched URL".into());
        }
    }

    let mut app_state = state.lock().await;

    // Check if state actually changed to avoid excessive disk I/O
    let should_save = if let Some(existing) = app_state.tabs.get(&tab_id) {
        existing.title != title || existing.url != url
    } else {
        true
    };

    if !should_save {
        return Ok(());
    }

    // Update or Insert TabState
    app_state.tabs.insert(
        tab_id.clone(),
        lotion_rs::state::TabState {
            id: tab_id.clone(),
            title: title.clone(),
            url: url.clone(),
            is_active: true,
            is_pinned: false,
        },
    );

    // Find which window this tab belongs to and update active_tab_id
    for window_state in app_state.windows.values_mut() {
        if window_state.tab_ids.contains(&tab_id) {
            window_state.active_tab_id = Some(tab_id.clone());
        }
    }

    NEEDS_SAVE.store(true, Ordering::Relaxed);
    tracing::debug!("Marked state for delayed save");
    Ok(())
}

#[tauri::command]
fn minimize_window(
    webview: tauri::Webview<tauri::Wry>,
    window_id: String,
    app: tauri::AppHandle<tauri::Wry>,
    config: tauri::State<'_, LotionConfig>,
) -> Result<(), String> {
    if !is_trusted_origin(&webview, &config) {
        return Err("Untrusted origin".into());
    }
    tracing::info!("CMD: minimize_window for {}", window_id);
    if let Some(window) = app.get_webview_window(&window_id) {
        window.minimize().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn maximize_window(
    webview: tauri::Webview<tauri::Wry>,
    window_id: String,
    app: tauri::AppHandle<tauri::Wry>,
    config: tauri::State<'_, LotionConfig>,
) -> Result<(), String> {
    if !is_trusted_origin(&webview, &config) {
        return Err("Untrusted origin".into());
    }
    tracing::info!("CMD: maximize_window for {}", window_id);
    if let Some(window) = app.get_webview_window(&window_id) {
        if let Ok(true) = window.is_maximized() {
            window.unmaximize().map_err(|e| e.to_string())?;
        } else {
            window.maximize().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn close_window(
    webview: tauri::Webview<tauri::Wry>,
    window_id: String,
    app: tauri::AppHandle<tauri::Wry>,
    config: tauri::State<'_, LotionConfig>,
) -> Result<(), String> {
    if !is_trusted_origin(&webview, &config) {
        return Err("Untrusted origin".into());
    }
    tracing::info!("CMD: close_window for {}", window_id);
    if let Some(window) = app.get_webview_window(&window_id) {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn log_network_event(
    webview: tauri::Webview<tauri::Wry>,
    _event: String,
    config: tauri::State<'_, LotionConfig>,
) -> Result<(), String> {
    if !is_trusted_origin(&webview, &config) {
        return Err("Untrusted origin".into());
    }

    // Privacy: Only log in debug builds to prevent sensitive URL/metadata leakage in prod logs.
    #[cfg(debug_assertions)]
    {
        // Truncate event to prevent log spamming or excessive memory usage
        let truncated_event = if _event.len() > 512 {
            format!("{}...", &_event[..512])
        } else {
            _event
        };
        tracing::debug!("[lotion-net] {}", truncated_event);
    }
    Ok(())
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("NO_AT_BRIDGE", "1");

        // KDE/Wayland Button Click Fix: Force native X11 Windowing instead of Wayland.
        // Tauri/WebKitGTK on Wayland often fails to route titlebar clicks properly to KWin.
        std::env::set_var("GDK_BACKEND", "x11");

        // Remove compositing and DMABUF restrictions that might interfere with native decorations
        // std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        // std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        std::env::set_var("WEBKIT_USE_SINGLE_WEB_PROCESS", "1");
        std::env::set_var("WEBKIT_DISABLE_ACCESSIBILITY", "1");
        std::env::set_var("GTK_A11Y", "none");
        std::env::set_var("GIO_USE_VFS", "local");
    }

    // Set RUST_LOG only if not already set by the user
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    tracing::info!("Starting Lotion-rs...");

    // Get or create application secret
    let app_secret =
        get_or_create_app_secret().expect("Failed to get or create application secret");
    let app_secret_arc = Arc::new(app_secret);

    // Load user config
    let config = LotionConfig::load();
    tracing::info!(
        "Config: theme={}, restore_tabs={}",
        config.active_theme,
        config.restore_tabs
    );

    // Load saved state (if any)
    let app_state = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(AppState::load_from_disk(&app_secret_arc))
        .unwrap_or_default();
    let app_state = Arc::new(tokio::sync::Mutex::new(app_state));

    // Initialize Concrete Modules
    let security = Arc::new(SecurityModule::new());
    let policy = Arc::new(PolicyManager::new());
    let theming = Arc::new(ThemeManager::with_config(
        &config.active_theme,
        config.custom_css_path.clone(),
    ));
    let tab_manager = Arc::new(lotion_rs::tab_manager::TabManager::<tauri::Wry>::new(
        security.litebox.clone(),
    ));

    // Tauri Application Context
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { .. } => {
                tracing::info!("WINDOW EVENT [{}]: CloseRequested", window.label());
            }
            tauri::WindowEvent::Focused(focused) => {
                tracing::debug!("WINDOW EVENT [{}]: Focused({})", window.label(), focused);
            }
            _ => {}
        })
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![
            lotion_rs::i18n::get_translation,
            lotion_rs::i18n::set_locale,
            lotion_rs::spellcheck::check_spelling,
            lotion_rs::spellcheck::get_spelling_suggestions,
            update_tab_state,
            get_window_tabs,
            switch_tab,
            close_tab,
            new_tab,
            minimize_window,
            maximize_window,
            close_window,
            log_network_event
        ])
        .setup(move |app| {
            // Initialize modules in Tauri state FIRST as trait objects where expected
            app.manage::<Arc<dyn lotion_rs::traits::SecuritySandbox>>(security.litebox.clone());
            app.manage::<Arc<dyn lotion_rs::traits::PolicyEnforcer>>(policy);
            app.manage::<Arc<dyn lotion_rs::traits::ThemingEngine<tauri::Wry>>>(theming);
            app.manage::<Arc<dyn lotion_rs::traits::TabOrchestrator<tauri::Wry>>>(tab_manager);
            app.manage(config);
            app.manage(app_state);
            app.manage(I18nManager::new());
            app.manage(SpellcheckManager::new());
            app.manage(app_secret_arc.clone()); // Manage the app_secret_arc

            let handle = app.handle().clone();

            // Native Menu Setup
            let _ = lotion_rs::menu::create_main_menu(&handle);

            // Consolidate State Management: Single background loop for disk I/O
            let state_save_handle = app.handle().clone();
            let state_save_secret = app_secret_arc.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(30)).await;

                    // Check both flags (main.rs and window_controller.rs)
                    let mut needs_save = NEEDS_SAVE.swap(false, Ordering::SeqCst);
                    needs_save |=
                        lotion_rs::window_controller::NEEDS_SAVE.swap(false, Ordering::SeqCst);

                    if needs_save {
                        let app_state_lock =
                            state_save_handle.state::<Arc<tokio::sync::Mutex<AppState>>>();
                        let app_state = app_state_lock.lock().await;
                        if let Err(e) = app_state.save_to_disk(&state_save_secret).await {
                            tracing::error!("[lotion-state] Background save failed: {}", e);
                        } else {
                            tracing::info!("[lotion-state] Background state save completed.");
                        }
                    }
                }
            });

            // Global Menu Event Handler
            handle.on_menu_event(move |app_handle, event| {
                match event.id.as_ref() {
                    "preferences" => {
                        tracing::info!("Menu: Preferences requested");
                        // Future: open preferences window
                    }
                    "quit" => {
                        app_handle.exit(0);
                    }
                    _ => {}
                }
            });

            let security_state = handle
                .state::<Arc<dyn lotion_rs::traits::SecuritySandbox>>()
                .inner()
                .clone();

            // Spawn the main window directly via Tauri WindowController
            match lotion_rs::window_controller::WindowController::<tauri::Wry>::new(
                &handle,
                security_state,
            ) {
                Ok(wc) => {
                    wc.setup_listeners(handle.clone());
                    if let Err(e) = wc.setup_tabs(&handle) {
                        tracing::error!("Failed to set up tabs: {}", e);
                    }
                    tracing::info!("WindowController initialized and set up.");
                }
                Err(e) => {
                    tracing::error!("Failed to create WindowController: {}", e);
                }
            }

            tracing::info!("Tauri background layer initialized.");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
