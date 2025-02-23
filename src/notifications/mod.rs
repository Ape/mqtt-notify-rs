pub mod desktop;
pub mod xmpp;

use async_trait::async_trait;
use futures::future;

#[async_trait]
pub trait NotificationPlugin {
    async fn notify(&self, title: &str, body: &str);
    async fn run(&self) {}
}

pub type DynNotificationPlugin = dyn NotificationPlugin + Send + Sync;

pub struct CompositeNotificationPlugin {
    plugins: Vec<Box<DynNotificationPlugin>>,
}

impl CompositeNotificationPlugin {
    pub fn new(plugins: Vec<Box<DynNotificationPlugin>>) -> Self {
        CompositeNotificationPlugin { plugins }
    }
}

#[async_trait]
impl NotificationPlugin for CompositeNotificationPlugin {
    async fn notify(&self, title: &str, body: &str) {
        let futures = self.plugins.iter().map(|plugin| plugin.notify(title, body));
        future::join_all(futures).await;
    }

    async fn run(&self) {
        let runs = self.plugins.iter().map(|plugin| plugin.run());
        future::join_all(runs).await;
    }
}

pub use desktop::DesktopNotificationPlugin;
pub use xmpp::XMPPNotificationPlugin;
