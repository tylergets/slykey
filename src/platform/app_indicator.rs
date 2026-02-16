use std::env;
use std::fs;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process;
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use gtk::prelude::*;
use libappindicator::{AppIndicator as LibAppIndicator, AppIndicatorStatus};

use crate::config::{MenuSnippet, NotificationConfig};
use crate::core::expansion::render_template_macros;
use crate::platform::dbus_notification;

pub struct AppIndicator {
    _gtk_thread: JoinHandle<()>,
}

const BUNDLED_TRAY_ICON_NAME: &str = "slykey";
const BUNDLED_TRAY_ICON_SVG: &[u8] = include_bytes!("slykey.svg");

pub fn start(
    snippets: Vec<MenuSnippet>,
    globals: HashMap<String, String>,
    notifications: NotificationConfig,
) -> Option<AppIndicator> {
    if env::var_os("DISPLAY").is_none() {
        eprintln!("warning: DISPLAY is not set; cannot create tray icon");
        return None;
    }
    if env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
        eprintln!("warning: DBus session is not set; appindicator may not be visible");
    }

    let (ready_tx, ready_rx) = mpsc::channel();
    let gtk_thread = std::thread::spawn(move || {
        if let Err(err) = run_indicator(ready_tx, snippets, globals, notifications) {
            eprintln!("tray thread exited: {err}");
        }
    });

    match ready_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => Some(AppIndicator {
            _gtk_thread: gtk_thread,
        }),
        Ok(Err(err)) => {
            eprintln!("failed to start tray icon: {err}");
            let _ = gtk_thread.join();
            None
        }
        Err(_) => {
            eprintln!("warning: tray startup timed out; keeping tray thread running");
            Some(AppIndicator {
                _gtk_thread: gtk_thread,
            })
        }
    }
}

fn run_indicator(
    ready_tx: Sender<Result<(), String>>,
    snippets: Vec<MenuSnippet>,
    globals: HashMap<String, String>,
    notifications: NotificationConfig,
) -> Result<(), String> {
    if let Err(err) = gtk::init() {
        let msg = err.to_string();
        let _ = ready_tx.send(Err(msg.clone()));
        return Err(msg);
    }

    let tray_icon_name = install_bundled_icon().unwrap_or("input-keyboard");
    let mut indicator = LibAppIndicator::new("slykey", tray_icon_name);
    indicator.set_title("slykey");
    indicator.set_status(AppIndicatorStatus::Active);

    let mut menu = gtk::Menu::new();
    let running_item = gtk::MenuItem::with_label("Running");
    running_item.set_sensitive(false);
    menu.append(&running_item);
    running_item.show();

    if !snippets.is_empty() {
        let separator = gtk::SeparatorMenuItem::new();
        menu.append(&separator);
        separator.show();
    }
    let has_snippets = !snippets.is_empty();

    let globals = Arc::new(globals);
    let notify_on_snippet_copy = notifications.on_snippet_copy;

    for snippet in snippets {
        let item = gtk::MenuItem::with_label(&snippet.title);
        let title = snippet.title;
        let content = snippet.content;
        let globals = Arc::clone(&globals);
        item.connect_activate(move |_| {
            let text = match render_template_macros(&content, &globals) {
                Ok(rendered) => rendered,
                Err(err) => {
                    eprintln!("failed to render snippet template macros: {err}");
                    content.clone()
                }
            };
            let clipboard = gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD);
            clipboard.set_text(&text);
            clipboard.store();

            if notify_on_snippet_copy {
                if let Err(err) = dbus_notification::send_notification("Copied Snippet", &title) {
                    eprintln!("failed to send snippet notification: {err}");
                }
            }
        });
        menu.append(&item);
        item.show();
    }

    if has_snippets {
        let separator = gtk::SeparatorMenuItem::new();
        menu.append(&separator);
        separator.show();
    }

    let quit_item = gtk::MenuItem::with_label("Quit");
    quit_item.connect_activate(|_| process::exit(0));
    menu.append(&quit_item);
    quit_item.show();

    menu.show_all();

    indicator.set_menu(&mut menu);
    let _ = ready_tx.send(Ok(()));

    gtk::main();
    Ok(())
}

fn install_bundled_icon() -> Option<&'static str> {
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".local/share")))?;

    let icon_path = data_home
        .join("icons")
        .join("hicolor")
        .join("scalable")
        .join("apps")
        .join(format!("{BUNDLED_TRAY_ICON_NAME}.svg"));

    if let Some(parent) = icon_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("warning: failed to create icon directory: {err}");
            return None;
        }
    }

    if let Err(err) = fs::write(&icon_path, BUNDLED_TRAY_ICON_SVG) {
        eprintln!("warning: failed to write bundled tray icon: {err}");
        return None;
    }

    Some(BUNDLED_TRAY_ICON_NAME)
}
