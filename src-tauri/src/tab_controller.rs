use crate::traits::{PolicyEnforcer, ThemingEngine};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime, Url, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

pub struct TabController<R: Runtime> {
    pub tab_id: String,
    pub window_id: String,
    pub webview: WebviewWindow<R>,
}

impl<R: Runtime> TabController<R> {
    pub fn new(
        app: &AppHandle<R>,
        window_id: &str,
        tab_id: String,
        url_str: &str,
        _litebox: Arc<crate::litebox::LiteBox>,
    ) -> tauri::Result<Self> {
        let policy = app.state::<Arc<dyn PolicyEnforcer>>().inner().clone();

        if !policy.validate_url(url_str) {
            return Err(tauri::Error::AssetNotFound(format!(
                "Zero-Trust Policy Blocked: {}",
                url_str
            )));
        }

        let window = app
            .get_webview_window(window_id)
            .ok_or(tauri::Error::AssetNotFound(format!(
                "Window {} not found",
                window_id
            )))?;

        let url = url_str
            .parse::<Url>()
            .map_err(|e| tauri::Error::AssetNotFound(e.to_string()))?;

        window.navigate(url.clone())?;
        window.show()?;
        window.set_focus()?;

        log::info!("Navigated window {} to tab: {} with URL: {}", window_id, tab_id, url_str);

        let theming = app.state::<Arc<dyn ThemingEngine<R>>>();
        let active_theme = theming.get_active_theme();
        theming.inject_theme(window.as_ref(), &active_theme);

        let platform_css = "#lotion-custom-titlebar { display: none !important; }";

        let title_observer_js = format!(
            "(function() {{
                const tabId = '{0}';
                let lastTitle = document.title;
                const observer = new MutationObserver(function() {{
                    if (document.title !== lastTitle) {{
                        lastTitle = document.title;
                        const currentUrl = window.location.href;
                        if (window.__TAURI__) {{
                            window.__TAURI__.invoke('update_tab_state', {{
                                tabId: tabId,
                                title: lastTitle,
                                url: currentUrl
                            }});
                        }}
                    }}
                }});
                observer.observe(document.querySelector('title') || document.documentElement, {{
                    subtree: true,
                    characterData: true,
                    childList: true
                }});
                const style = document.createElement('style');
                style.textContent = `{1}`;
                document.head.appendChild(style);
            }})();",
            tab_id, platform_css
        );

        let _ = window.eval(&title_observer_js);

        Ok(Self {
            tab_id,
            window_id: window_id.to_string(),
            webview: window,
        })
    }

    pub fn destroy(&self) -> tauri::Result<()> {
        log::info!("Cleaning tab context: {}", self.tab_id);
        let _ = self.webview.navigate("about:blank".parse().unwrap());
        Ok(())
    }
}

pub fn spawn_secure_popup<R: Runtime>(
    app: &AppHandle<R>,
    _policy: Arc<dyn PolicyEnforcer>,
    url: Url,
) {
    if let Some(orchestrator) = app.try_state::<Arc<dyn crate::traits::TabOrchestrator<R>>>() {
        if let Err(e) = orchestrator.inner().create_tab(app, "main", url.as_str()) {
            log::error!("Zero-Trust: Failed to route popup: {}", e);
        }
    }
}
