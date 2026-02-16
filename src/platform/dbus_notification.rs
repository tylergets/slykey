use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use dbus::arg::{RefArg, Variant};
use dbus::blocking::Connection;

pub fn send_notification(summary: &str, body: &str) -> Result<()> {
    let connection = Connection::new_session().context("failed to connect to D-Bus session")?;
    let proxy = connection.with_proxy(
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        Duration::from_millis(800),
    );

    let actions: Vec<&str> = Vec::new();
    let hints: HashMap<&str, Variant<Box<dyn RefArg>>> = HashMap::new();

    let _: (u32,) = proxy
        .method_call(
            "org.freedesktop.Notifications",
            "Notify",
            (
                "",
                0u32,
                "",
                summary,
                body,
                actions,
                hints,
                2000i32,
            ),
        )
        .context("failed to send desktop notification")?;

    Ok(())
}
