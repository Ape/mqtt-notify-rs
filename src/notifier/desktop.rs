use async_trait::async_trait;

use crate::notifier::Notifier;

pub struct DesktopNotifier;

impl DesktopNotifier {
    pub fn new() -> Self {
        DesktopNotifier
    }
}

#[async_trait]
impl Notifier for DesktopNotifier {
    async fn notify(&self, title: &str, body: &str) {
        let _ = notify_rust::Notification::new()
            .summary(title)
            .body(body)
            .timeout(20000) // milliseconds
            .show();
    }
}
