use std::sync::Arc;
use tauri::{
    menu::{AboutMetadata, MenuBuilder, MenuItem, SubmenuBuilder},
    AppHandle, Manager, Runtime,
};

pub fn create_main_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let pkg_info = app.package_info();

    // lotion-rs Menu
    let lotion_menu = SubmenuBuilder::new(app, "lotion-rs")
        .about(Some(AboutMetadata {
            name: Some("lotion-rs".to_string()),

            version: Some(pkg_info.version.to_string()),
            ..Default::default()
        }))
        .separator()
        .quit()
        .build()?;

    // Navigation Menu (Temporarily simplified or removed if not handled yet)
    // Removed Back/Forward/Refresh/Home as they are not currently linked to handlers in this view.

    // Edit Menu
    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    // Theme Submenu
    let theme_submenu = SubmenuBuilder::new(app, "Theme")
        .item(&MenuItem::with_id(
            app,
            "theme_light",
            "Light (Default)",
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(
            app,
            "theme_dracula",
            "Dracula",
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

    // View Menu
    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&MenuItem::with_id(
            app,
            "reload",
            "Reload",
            true,
            Some("F5"),
        )?)
        .separator()
        .item(&theme_submenu)
        .separator()
        .item(&MenuItem::with_id(
            app,
            "toggle_dev_tools",
            "Toggle Developer Tools",
            true,
            Some("F12"),
        )?)
        .separator()
        .item(&MenuItem::with_id(
            app,
            "toggle_menu_bar",
            "Toggle Menu Bar",
            true,
            Some("CmdOrCtrl+Shift+M"),
        )?)
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&lotion_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .build()?;

    app.set_menu(menu)?;

    app.on_menu_event(move |app_handle, event| {
        match event.id().as_ref() {
            "toggle_dev_tools" => {
                log::info!("Menu: Toggle Developer Tools (disabled in release)");
            }
            theme_id if theme_id.starts_with("theme_") => {
                let theme_name = theme_id.replace("theme_", "");
                log::info!("Menu: Switch theme to {}", theme_name);

                // Retrieve state manually to avoid borrowing issues
                if let Some(theming) =
                    app_handle.try_state::<Arc<dyn crate::traits::ThemingEngine<R>>>()
                {
                    theming.set_active_theme(&theme_name);

                    if let Some(orchestrator) =
                        app_handle.try_state::<Arc<dyn crate::traits::TabOrchestrator<R>>>()
                    {
                        let tab_ids = orchestrator.get_tab_ids();
                        for tab_id in tab_ids {
                            let _ = orchestrator.inject_theme_into_tab(
                                app_handle,
                                &tab_id,
                                &theme_name,
                            );
                        }
                    }

                    // Persist to config
                    if let Some(config_state) =
                        app_handle.try_state::<crate::config::LotionConfig>()
                    {
                        let mut config = config_state.inner().clone();
                        config.active_theme = theme_name;
                        let _ = config.save();
                    }
                }
            }
            _ => {}
        }
    });

    Ok(())
}
