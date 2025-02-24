use async_trait::async_trait;
use notify_rust::Timeout;

use crate::notifier::Notifier;

pub struct DesktopNotifier;

impl DesktopNotifier {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Notifier for DesktopNotifier {
    async fn notify(&self, title: &str, body: &str) {
        let notification = notify_rust::Notification::new()
            .summary(title)
            .body(body)
            .timeout(Timeout::Milliseconds(20000))
            .finalize();

        if let Err(e) = notification.show() {
            log::error!("Desktop notification failed: {}", e);
        }
    }
}
