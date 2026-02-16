use ksni::blocking::{Handle, TrayMethods};

pub struct AppIndicator {
    _handle: Handle<SlykeyTray>,
}

pub fn start() -> Option<AppIndicator> {
    let tray = SlykeyTray;
    match tray.assume_sni_available(true).spawn() {
        Ok(handle) => Some(AppIndicator { _handle: handle }),
        Err(err) => {
            eprintln!("failed to start app indicator: {err}");
            None
        }
    }
}

struct SlykeyTray;

impl ksni::Tray for SlykeyTray {
    fn id(&self) -> String {
        "slykey".into()
    }

    fn title(&self) -> String {
        "slykey".into()
    }

    fn icon_name(&self) -> String {
        "input-keyboard".into()
    }
}
