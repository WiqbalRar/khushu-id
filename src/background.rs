use crate::i18n::tr;
use adw::prelude::*;
use ksni::{MenuItem, Tray};
use libadwaita as adw;
use std::sync::{Arc, OnceLock, RwLock};

struct KhushuTrayData {
    open_label: String,
    quit_label: String,
}

struct KhushuTray {
    data: Arc<RwLock<KhushuTrayData>>,
}

static TRAY_DATA: OnceLock<Arc<RwLock<KhushuTrayData>>> = OnceLock::new();
static TRAY_HANDLE: OnceLock<ksni::Handle<KhushuTray>> = OnceLock::new();

fn get_tray_data() -> Arc<RwLock<KhushuTrayData>> {
    TRAY_DATA
        .get_or_init(|| {
            Arc::new(RwLock::new(KhushuTrayData {
                open_label: String::new(),
                quit_label: String::new(),
            }))
        })
        .clone()
}

impl Tray for KhushuTray {
    fn icon_name(&self) -> String {
        "io.github.sniper1720.khushu-symbolic".into()
    }

    fn id(&self) -> String {
        "io.github.sniper1720.khushu".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        Vec::new()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        gtk4::glib::idle_add(move || {
            if let Some(app) = gtk4::gio::Application::default() {
                app.activate();
            }
            gtk4::glib::ControlFlow::Break
        });
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        use ksni::menu::*;
        let data = self.data.read().unwrap();
        vec![
            StandardItem {
                label: data.open_label.clone(),
                activate: Box::new(|_this: &mut Self| {
                    gtk4::glib::idle_add(move || {
                        if let Some(app) = gtk4::gio::Application::default() {
                            app.activate();
                        }
                        gtk4::glib::ControlFlow::Break
                    });
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: data.quit_label.clone(),
                activate: Box::new(|_this: &mut Self| {
                    gtk4::glib::idle_add(move || {
                        if let Some(app) = gtk4::gio::Application::default() {
                            use gtk4::prelude::*;
                            app.quit();
                        }
                        gtk4::glib::ControlFlow::Break
                    });
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

async fn request_background_portal() -> Result<(), ashpd::Error> {
    use ashpd::desktop::background::Background;
    let config = crate::config::AppConfig::load();

    let response = Background::request()
        .reason("Khushu needs to run in the background to send prayer time notifications")
        .auto_start(config.autostart)
        .command(&["khushu", "--background"])
        .dbus_activatable(false)
        .send()
        .await?
        .response()?;

    log::info!(
        "Background portal: auto_start={}, background={}",
        response.auto_start(),
        response.run_in_background()
    );
    Ok(())
}

async fn setup_tray_icon() {
    use ksni::TrayMethods;
    use std::sync::atomic::{AtomicBool, Ordering};

    static TRAY_SPAWNED: AtomicBool = AtomicBool::new(false);
    if TRAY_SPAWNED.swap(true, Ordering::SeqCst) {
        return;
    }

    let lang = std::env::var("LANGUAGE").unwrap_or_default();
    let lang_ref = if lang.is_empty() { "en" } else { &lang };

    let data = get_tray_data();
    {
        let mut d = data.write().unwrap();
        d.open_label = tr("Open Khushu", lang_ref);
        d.quit_label = tr("Quit", lang_ref);
    }

    let tray = KhushuTray { data };

    let is_sandboxed = std::path::Path::new("/.flatpak-info").exists();

    match tray.disable_dbus_name(is_sandboxed).spawn().await {
        Ok(handle) => {
            let _ = TRAY_HANDLE.set(handle);
            tokio::spawn(async move {
                std::future::pending::<()>().await;
            });
        }
        Err(e) => {
            log::error!("Failed to spawn KSNI tray icon: {}", e);
        }
    }
}

pub fn update_tray_labels(lang: &str) {
    let data = get_tray_data();
    {
        let mut d = data.write().unwrap();
        d.open_label = tr("Open Khushu", lang);
        d.quit_label = tr("Quit", lang);
    }

    if let Some(handle) = TRAY_HANDLE.get().cloned() {
        tokio::spawn(async move {
            let _ = handle.update(|_| {}).await;
        });
    }
}

pub fn setup_background() {
    let is_sandboxed =
        std::path::Path::new("/.flatpak-info").exists() || std::env::var_os("SNAP").is_some();

    gtk4::glib::spawn_future_local(async move {
        if is_sandboxed && let Err(e) = request_background_portal().await {
            log::info!("Background portal failed: {e}");
        }

        setup_tray_icon().await;
    });
}
