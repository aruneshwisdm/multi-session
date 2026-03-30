use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

static ACTION_TX: OnceLock<flume::Sender<String>> = OnceLock::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Returns a receiver for session IDs from notification clicks.
/// Call once at startup before `init()`.
pub fn action_receiver() -> flume::Receiver<String> {
    let (tx, rx) = flume::unbounded();
    let _ = ACTION_TX.set(tx);
    rx
}

/// Initialize the notification system.
/// On Linux, this is a no-op (D-Bus notifications don't require pre-authorization).
/// On Windows, this is a no-op (toast notifications don't require pre-authorization).
pub fn init() {
    INITIALIZED.store(true, Ordering::Relaxed);
    #[cfg(target_os = "linux")]
    {
        // Verify D-Bus is available by attempting a test notification
        match notify_rust::Notification::new()
            .summary("jc")
            .body("Notification system ready")
            .timeout(1000)
            .show()
        {
            Ok(_) => log::info!("notify: D-Bus notification system available"),
            Err(e) => log::warn!("notify: D-Bus not available, notifications disabled: {e}"),
        }
    }
}

/// Post a notification.
///
/// - `title`: notification title
/// - `message`: notification body
/// - `critical`: if true, uses urgency=critical (Linux) or persistent toast (Windows)
/// - `session_id`: optional session identifier for click routing
pub fn notify(title: &str, message: &str, critical: bool, session_id: Option<&str>) {
    eprintln!("notify: {title} — {message}");

    if !INITIALIZED.load(Ordering::Relaxed) {
        return;
    }

    let title = title.to_string();
    let message = message.to_string();
    let _session_id = session_id.map(str::to_string);

    std::thread::spawn(move || {
        if let Err(e) = post_notification(&title, &message, critical) {
            log::warn!("notify: failed to post notification: {e}");
        }
    });
}

#[cfg(target_os = "linux")]
fn post_notification(title: &str, message: &str, critical: bool) -> anyhow::Result<()> {
    use notify_rust::Urgency;

    let urgency = if critical { Urgency::Critical } else { Urgency::Normal };

    notify_rust::Notification::new()
        .summary(title)
        .body(message)
        .urgency(urgency)
        .show()?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn post_notification(title: &str, message: &str, _critical: bool) -> anyhow::Result<()> {
    notify_rust::Notification::new()
        .summary(title)
        .body(message)
        .show()?;

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn post_notification(title: &str, message: &str, _critical: bool) -> anyhow::Result<()> {
    // Fallback: just log to stderr (already done in notify())
    let _ = (title, message);
    Ok(())
}
