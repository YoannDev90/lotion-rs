use crate::traits::SecuritySandbox;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime, WebviewWindow, WebviewWindowBuilder as WindowBuilder};

use std::sync::atomic::{AtomicBool, Ordering};

pub static NEEDS_SAVE: AtomicBool = AtomicBool::new(false);

pub struct WindowController<R: Runtime> {
    pub window: WebviewWindow<R>,
    pub security: Arc<dyn SecuritySandbox>,
}

impl<R: Runtime> WindowController<R> {
    pub fn new(app: &AppHandle<R>, security: Arc<dyn SecuritySandbox>) -> tauri::Result<Self> {
        // Create the main window with native decorations.
        #[allow(unused_mut)]
        let mut window_builder = WindowBuilder::new(
            app,
            "main",
            tauri::WebviewUrl::External("about:blank".parse().unwrap()),
        )
        .title("lotion-rs")
        .inner_size(1200.0, 768.0)
        .center()
        .focused(true)
        .visible(false);

        #[cfg(not(target_os = "macos"))]
        {
            window_builder = window_builder.decorations(true).shadow(true);
            log::debug!("Window native decorations enabled for target_os");

            // KDE/Linux specific: Ensure the icon is explicitly set from the assets
            // and set the app_id to match the .desktop file for Wayland support.
            let icon_path = std::path::PathBuf::from("assets/icon.png");
            let icon = tauri::image::Image::from_path(icon_path).ok();
            let icon = icon.or_else(|| app.default_window_icon().cloned());

            if let Some(i) = icon {
                window_builder = window_builder.icon(i).unwrap_or_else(|e| {
                    log::warn!("Failed to set icon: {}", e);
                    WindowBuilder::new(
                        app,
                        "main",
                        tauri::WebviewUrl::External("about:blank".parse().unwrap()),
                    )
                    .title("lotion-rs")
                    .inner_size(1200.0, 768.0)
                    .center()
                    .focused(true)
                    .visible(false)
                    .decorations(true)
                    .shadow(true)
                });
            }
        }

        let window = window_builder.build()?;

        let app_state_lock = app.state::<Arc<tokio::sync::Mutex<crate::state::AppState>>>();

        // Fix for Linux/KDE: The native window decoration needs to be "activated"
        // to receive clicks on its buttons without a prior click to focus.
        #[cfg(target_os = "linux")]
        {
            let win = window.clone();
            tauri::async_runtime::spawn(async move {
                // Wait for the webview to be semi-ready
                tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

                // Sequence to force KWin to register the decoration state
                let _ = win.show();
                let _ = win.unminimize(); // Ensure it's not starting minimized
                let _ = win.set_focus();

                // Emitting a resize often forces a layout/event recalculation in GTK/KWin
                if let Ok(size) = win.inner_size() {
                    let _ = win.set_size(tauri::Size::Physical(size));
                }

                log::info!("Linux: Native window activation sequence completed");
            });
        }
        #[cfg(not(target_os = "linux"))]
        {
            window.show()?;
        }
        let mut app_state = app_state_lock.blocking_lock();
        if !app_state.windows.contains_key("main") {
            app_state.windows.insert(
                "main".to_string(),
                crate::state::WindowState {
                    id: "main".to_string(),
                    bounds: crate::state::Bounds {
                        x: None,
                        y: None,
                        width: 1200.0,
                        height: 768.0,
                    },
                    is_focused: true,
                    is_maximized: false,
                    is_minimized: false,
                    is_full_screen: false,
                    tab_ids: Vec::new(),
                    active_tab_id: None,
                },
            );
            NEEDS_SAVE.store(true, Ordering::Relaxed);
        }

        Ok(Self { window, security })
    }

    pub fn setup_listeners(&self, app_handle: AppHandle<R>) {
        let window_label = self.window.label().to_string();
        let handle_for_close = app_handle.clone();

        self.window.on_window_event(move |event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                log::info!("Window {} close requested", window_label);
                api.prevent_close();
                handle_for_close.exit(0);
            }
            tauri::WindowEvent::Focused(focused) => {
                log::debug!("Window {} focused: {}", window_label, focused);
                let app_state_lock =
                    app_handle.state::<Arc<tokio::sync::Mutex<crate::state::AppState>>>();
                let mut app_state = app_state_lock.blocking_lock();
                if *focused {
                    app_state.focused_window_id = Some(window_label.clone());
                }
                if let Some(w_state) = app_state.windows.get_mut(&window_label) {
                    w_state.is_focused = *focused;
                }
                NEEDS_SAVE.store(true, Ordering::Relaxed);
            }
            tauri::WindowEvent::Resized(size) => {
                log::debug!("Window {} resized to {:?}", window_label, size);
                let app_state_lock =
                    app_handle.state::<Arc<tokio::sync::Mutex<crate::state::AppState>>>();
                let mut app_state = app_state_lock.blocking_lock();
                if let Some(w_state) = app_state.windows.get_mut(&window_label) {
                    w_state.bounds.width = size.width as f64;
                    w_state.bounds.height = size.height as f64;
                }
                NEEDS_SAVE.store(true, Ordering::Relaxed);
            }
            tauri::WindowEvent::Moved(position) => {
                log::debug!("Window {} moved to {:?}", window_label, position);
                let app_state_lock =
                    app_handle.state::<Arc<tokio::sync::Mutex<crate::state::AppState>>>();
                let mut app_state = app_state_lock.blocking_lock();
                if let Some(w_state) = app_state.windows.get_mut(&window_label) {
                    w_state.bounds.x = Some(position.x as f64);
                    w_state.bounds.y = Some(position.y as f64);
                }
                NEEDS_SAVE.store(true, Ordering::Relaxed);
            }
            _ => {}
        });
    }

    pub fn setup_tabs(&self, app: &AppHandle<R>) -> tauri::Result<()> {
        let app_state_lock = app.state::<Arc<tokio::sync::Mutex<crate::state::AppState>>>();
        let app_state = app_state_lock.blocking_lock();

        if let Some(w_state) = app_state.windows.get("main") {
            if let Some(ref active_tab_id) = w_state.active_tab_id {
                if let Some(tab_state) = app_state.tabs.get(active_tab_id) {
                    if let Some(orchestrator) =
                        app.try_state::<Arc<dyn crate::traits::TabOrchestrator<R>>>()
                    {
                        let _ = orchestrator.create_tab(app, "main", &tab_state.url);
                    }
                }
            } else {
                if let Some(orchestrator) =
                    app.try_state::<Arc<dyn crate::traits::TabOrchestrator<R>>>()
                {
                    let _ = orchestrator.create_tab(app, "main", "https://www.notion.so");
                }
            }
        }
        Ok(())
    }
}
