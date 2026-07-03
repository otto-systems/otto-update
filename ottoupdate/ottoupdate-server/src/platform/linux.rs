#[cfg(target_os = "linux")]
pub async fn notify_ready() {
    let _ = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]);
}

#[cfg(target_os = "linux")]
pub async fn watchdog_tick() {
    let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Watchdog]);
}

#[cfg(not(target_os = "linux"))]
pub async fn notify_ready() {}

#[cfg(not(target_os = "linux"))]
pub async fn watchdog_tick() {}
