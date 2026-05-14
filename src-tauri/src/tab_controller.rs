use crate::litebox::LiteBox;
use crate::traits::{PolicyEnforcer, ThemingEngine};
use std::sync::Arc;
use tauri::webview::{NewWindowResponse, Webview, WebviewBuilder};
use tauri::{AppHandle, Manager, Runtime, Url, WebviewUrl};
use tauri_plugin_opener::OpenerExt;

pub struct TabController<R: Runtime> {
    pub tab_id: String,
    pub window_id: String,
    pub webview: Webview<R>,
}

impl<R: Runtime> TabController<R> {
    pub fn new(
        app: &AppHandle<R>,
        window_id: &str,
        tab_id: String,
        url_str: &str,
        _litebox: Arc<LiteBox>,
    ) -> tauri::Result<Self> {
        let policy = app.state::<Arc<dyn PolicyEnforcer>>().inner().clone();

        // Zero-Trust Enforcement: Validate URL before creation
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

        // Create a new webview for this tab using the secure factory
        let webview_builder =
            create_secure_webview_builder(app, &tab_id, &url, window_id, policy.clone());

        // Set background to transparent to hide the parent "Initialzing" text
        let webview_builder = webview_builder.transparent(true);

        let inner_size = window.inner_size()?;
        // In Tauri 2.0 WebviewWindow can ADD CHILD using its Window part
        let webview = window.as_ref().window().add_child(
            webview_builder,
            tauri::LogicalPosition::new(0.0, 0.0), // Use (0,0) as we want it to cover the parent
            tauri::LogicalSize::new(inner_size.width as f64, inner_size.height as f64),
        )?;

        // Ensure the child webview is shown and covers the parent completely.
        webview.show()?;
        webview.set_focus()?;

        // Hide the parent window's background content and show the final result
        let _ = window.as_ref().window().set_focus();
        let _ = window.as_ref().window().show();

        log::info!("Created tab webview: {} in window: {}", tab_id, window_id);

        // Inject theme from config (not hardcoded)
        let theming = app.state::<Arc<dyn ThemingEngine<R>>>();
        let active_theme = theming.get_active_theme();
        theming.inject_theme(&webview, &active_theme);

        // Inject title observer and platform-specific Window Controls
        // On Linux/Windows, we prefer native decorations. On macOS, we use custom ones.
        let platform_css = if cfg!(target_os = "macos") {
            ".notion-sidebar-container { margin-top: 38px !important; } .notion-topbar { padding-left: 80px !important; } #lotion-custom-titlebar { display: flex !important; }"
        } else {
            "#lotion-custom-titlebar { display: none !important; }"
        };

        let title_observer_js = format!(
            "(function() {{
                const tabId = '{0}';

                // 1. Title Observer
                let lastTitle = document.title;
                const observer = new MutationObserver(function() {{
                    if (document.title !== lastTitle) {{
                        lastTitle = document.title;
                        const currentUrl = window.location.href;
                        // Avoid redundant updates if nothing changed
                        if (window.__TAURI__) {{
                            window.__TAURI__.invoke('update_tab_state', {{
                                tabId: tabId,
                                title: lastTitle,
                                url: currentUrl
                            }});
                        }}
                    }}
                }});
                observer.observe(document.querySelector('title') || document.head, {{
                    subtree: true, characterData: true, childList: true
                }});

                // 2. Inject Native-feeling Window Controls (Titlebar) - ONLY FOR MACOS
                window.addEventListener('DOMContentLoaded', () => {{
                    const titlebar = document.createElement('div');
                    titlebar.id = 'lotion-custom-titlebar';
                    titlebar.setAttribute('data-tauri-drag-region', '');
                    titlebar.style.cssText = 'position: fixed; top: 0; left: 0; width: 100%; height: 38px; z-index: 999999; display: none; align-items: center; padding-left: 12px; pointer-events: none; background: inherit; border-bottom: 1px solid rgba(0,0,0,0.1);';

                    const style = document.createElement('style');
                    style.textContent = '{2}';
                    document.head.appendChild(style);

                    if (window.getComputedStyle(titlebar).display !== 'none') {{
                        const btnContainer = document.createElement('div');
                        btnContainer.style.cssText = 'display: flex; gap: 8px; align-items: center; pointer-events: auto;';

                        const createBtn = (color, clickHandler, label = '') => {{
                            const btn = document.createElement('div');
                            btn.style.cssText = 'width: 12px; height: 12px; border-radius: 50%; background-color: ' + color + '; cursor: pointer; border: 1px solid rgba(0,0,0,0.1); display: flex; align-items: center; justify-content: center; font-size: 8px; font-family: sans-serif;';
                            if (label) btn.innerText = label;
                            btn.addEventListener('click', (e) => {{
                                e.stopPropagation();
                                clickHandler();
                            }});
                            return btn;
                        }};

                        const closeBtn = createBtn('#ff5f56', () => {{
                            if (window.__TAURI__) {{
                                window.__TAURI__.invoke('close_window', {{ windowId: '{1}' }});
                            }}
                        }});

                        const minBtn = createBtn('#ffbd2e', () => {{
                            if (window.__TAURI__) {{
                                window.__TAURI__.invoke('minimize_window', {{ windowId: '{1}' }});
                            }}
                        }});

                        const maxBtn = createBtn('#27c93f', () => {{
                            if (window.__TAURI__) {{
                                window.__TAURI__.invoke('maximize_window', {{ windowId: '{1}' }});
                            }}
                        }});

                        btnContainer.appendChild(closeBtn);
                        btnContainer.appendChild(minBtn);
                        btnContainer.appendChild(maxBtn);

                        const spacer = document.createElement('div');
                        spacer.style.width = '24px';
                        btnContainer.appendChild(spacer);
                        titlebar.appendChild(btnContainer);
                    }}

                    const tabList = document.createElement('div');
                    tabList.style.cssText = 'display: flex; gap: 4px; align-items: flex-end; height: 100%; padding-top: 8px; pointer-events: auto;';

                    const renderTabs = async () => {{
                        if (!window.__TAURI__) return;
                        const tabs = await window.__TAURI__.invoke('get_window_tabs', {{ windowId: '{1}' }});
                        tabList.innerHTML = '';
                        tabs.forEach(t => {{
                            const tabEl = document.createElement('div');
                            tabEl.className = 'lotion-tab' + (t.id === tabId ? ' active' : '');
                            tabEl.innerText = t.title || 'Notion';
                            tabEl.onclick = () => {{
                                if (t.id !== tabId) {{
                                    window.__TAURI__.invoke('switch_tab', {{ tabId: t.id }});
                                }}
                            }};
                            tabList.appendChild(tabEl);
                        }});
                    }};

                    titlebar.appendChild(tabList);
                    document.body.appendChild(titlebar);
                    renderTabs();
                    // Removed performance-draining interval. Updates are now event-driven.
                    // setInterval(renderTabs, 5000);
                }});
            }})();",
            tab_id, window_id, platform_css
        );
        let _ = webview.eval(&title_observer_js);

        // Inject network monitor
        let network_monitor_js = r#"
            (function() {
                const log = (msg) => {
                    console.log(msg);
                    if (window.__TAURI__) {
                        window.__TAURI__.invoke("log_network_event", { event: msg });
                    }
                };

                const getOrigin = (url) => {
                    try {
                        const u = new URL(url);
                        return u.protocol + "//" + u.hostname;
                    } catch {
                        return "invalid-url";
                    }
                };

                const originalFetch = window.fetch;
                window.fetch = async (...args) => {
                    const url = args[0] instanceof Request ? args[0].url : args[0];
                    const origin = getOrigin(url);
                    try {
                        const response = await originalFetch(...args);
                        log("FETCH SUCCESS: " + response.status + " from " + origin);
                        return response;
                    } catch (error) {
                        log("FETCH ERROR: " + origin + " - " + error.message);
                        throw error;
                    }
                };

                const originalOpen = XMLHttpRequest.prototype.open;
                XMLHttpRequest.prototype.open = function(method, url) {
                    this._url = url;
                    const origin = getOrigin(url);
                    this.addEventListener("load", function() {
                        log("XHR SUCCESS: " + this.status + " from " + origin);
                    });
                    this.addEventListener("error", function() {
                        log("XHR ERROR: " + origin);
                    });
                    return originalOpen.apply(this, arguments);
                };
                log("Network monitoring active.");
            })();
        "#;
        let _ = webview.eval(network_monitor_js);

        Ok(Self {
            tab_id,
            window_id: window_id.to_string(),
            webview,
        })
    }

    pub fn destroy(&self) -> tauri::Result<()> {
        log::info!("Destroying tab: {}", self.tab_id);
        self.webview.close()?;
        Ok(())
    }
}

pub fn spawn_secure_popup<R: Runtime>(
    app: &AppHandle<R>,
    _policy: Arc<dyn PolicyEnforcer>,
    url: Url,
) {
    log::info!(
        "Intercepted popup request. Routing into a secure in-app tab: {}",
        url.as_str()
    );

    if let Some(orchestrator) = app.try_state::<Arc<dyn crate::traits::TabOrchestrator<R>>>() {
        if let Err(e) = orchestrator.inner().create_tab(app, "main", url.as_str()) {
            log::error!("Zero-Trust: Failed to route popup into managed tab: {}", e);
        }
    } else {
        log::error!("Zero-Trust: Cannot spawn tab securely. TabOrchestrator missing from state.");
    }
}

pub fn create_secure_webview_builder<R: Runtime>(
    app: &AppHandle<R>,
    label: &str,
    url: &Url,
    window_id: &str,
    policy: Arc<dyn PolicyEnforcer>,
) -> WebviewBuilder<R> {
    let webview_builder = WebviewBuilder::new(label, WebviewUrl::External(url.clone()));

    let _nav_app = app.clone();
    let nav_policy = policy.clone();
    let popup_app = app.clone();
    let popup_policy = policy.clone();
    let window_id_owned = window_id.to_string();

    webview_builder
        .on_navigation(move |url| {
            let _window_id = &window_id_owned;
            let url_str = url.as_str();

            if url_str.starts_with("lotion-action://") {
                log::warn!("SECURITY ALERT: External content attempted to trigger 'lotion-action://' via navigation. Blocking.");
                return false;
            }

            if !nav_policy.validate_url(url_str) {
                log::warn!("Zero-Trust: Blocked navigating to unauthorized domain: {}", url_str);
                return false;
            }
            true
        })
        .on_new_window(move |url, _builder| {
            let url_str = url.as_str();
            if popup_policy.should_route_popup_to_system_browser(url_str) {
                log::info!("Zero-Trust: Routing popup to system browser: {}", url_str);
                let _ = popup_app.opener().open_url(url_str, None::<String>);
                return NewWindowResponse::Deny;
            }

            if popup_policy.validate_url(url_str) {
                spawn_secure_popup(&popup_app, popup_policy.clone(), url.clone());
            } else {
                log::warn!("Zero-Trust: Blocked unauthorized popup request: {}", url_str);
            }
            NewWindowResponse::Deny
        })
}
