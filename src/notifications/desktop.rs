use async_trait::async_trait;

use crate::notifications::NotificationPlugin;

pub struct DesktopNotificationPlugin;

impl DesktopNotificationPlugin {
    pub fn new() -> Self {
        DesktopNotificationPlugin
    }
}

#[async_trait]
impl NotificationPlugin for DesktopNotificationPlugin {
    async fn notify(&self, title: &str, body: &str) {
        let _ = notify_rust::Notification::new()
            .summary(title)
            .body(body)
            .timeout(20000) // milliseconds
            .show();
    }
}
