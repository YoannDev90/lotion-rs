use std::sync::{Arc, RwLock};

use tauri::{
    menu::{MenuBuilder, MenuItem, SubmenuBuilder},
    AppHandle, Manager, Runtime,
};

pub fn create_main_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    // Build the View submenu (themes, language, dev tools, reload/fullscreen)

    let theme_submenu = SubmenuBuilder::new(app, "Theme")
        .item(&MenuItem::with_id(
            app,
            "theme_light",
            "Light",
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            app,
            "theme_dracula",
            "Dracula (Default)",
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            app,
            "theme_nord",
            "Nord",
            true,
            None::<&str>,
        )?)
        .build()?;

    let lang_submenu = SubmenuBuilder::new(app, "Language")
        .item(&MenuItem::with_id(
            app,
            "lang_en_US",
            "English",
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            app,
            "lang_fr_FR",
            "Français",
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            app,
            "lang_es_ES",
            "Español",
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            app,
            "lang_de_DE",
            "Deutsch",
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            app,
            "lang_ja_JP",
            "日本語",
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            app,
            "lang_zh_CN",
            "中文",
            true,
            None::<&str>,
        )?)
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&MenuItem::with_id(
            app,
            "reload",
            "Reload",
            true,
            Some("CmdOrCtrl+R"),
        )?)
        .item(&MenuItem::with_id(
            app,
            "toggle_fullscreen",
            "Toggle Fullscreen",
            true,
            Some("F11"),
        )?)
        .separator()
        .item(&theme_submenu)
        .item(&lang_submenu)
        .separator()
        .item(&MenuItem::with_id(
            app,
            "toggle_dev_tools",
            "Developer Tools",
            true,
            Some("F12"),
        )?)
        .build()?;

    let menu = MenuBuilder::new(app).item(&view_menu).build()?;

    app.set_menu(menu)?;

    app.on_menu_event(move |app_handle, event| match event.id.as_ref() {
        "reload" => {
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.eval("window.location.reload();");
            }
        }
        "toggle_fullscreen" => {
            if let Some(window) = app_handle.get_webview_window("main") {
                let is_fs = window.is_fullscreen().unwrap_or(false);
                let _ = window.set_fullscreen(!is_fs);
            }
        }
        "toggle_dev_tools" => {
            if let Some(window) = app_handle.get_webview_window("main") {
                if window.is_devtools_open() {
                    let _ = window.close_devtools();
                } else {
                    let _ = window.open_devtools();
                }
            }
        }
        theme_id if theme_id.starts_with("theme_") => {
            let theme_name = theme_id.replace("theme_", "");
            if let Some(theming) =
                app_handle.try_state::<Arc<dyn crate::traits::ThemingEngine<R>>>()
            {
                theming.set_active_theme(&theme_name);

                if let Some(orchestrator) =
                    app_handle.try_state::<Arc<dyn crate::traits::TabOrchestrator<R>>>()
                {
                    let tab_ids = orchestrator.get_tab_ids();
                    for tab_id in tab_ids {
                        let _ =
                            orchestrator.inject_theme_into_tab(app_handle, &tab_id, &theme_name);
                    }
                }

                if let Some(config_state) =
                    app_handle.try_state::<Arc<RwLock<crate::config::LotionConfig>>>()
                {
                    if let Ok(mut config) = config_state.write() {
                        config.active_theme = theme_name;
                        let _ = config.save();
                    }
                }
            }
        }
        lang_id if lang_id.starts_with("lang_") => {
            tracing::info!("Menu: Switch language to {}", lang_id.replace("lang_", ""));
        }
        _ => {}
    });

    Ok(())
}
