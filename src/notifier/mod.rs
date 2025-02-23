pub mod desktop;
pub mod xmpp;

use async_trait::async_trait;
use futures::future;

#[async_trait]
pub trait Notifier {
    async fn notify(&self, title: &str, body: &str);
    async fn run(&self) {}
}

pub type DynNotifier = dyn Notifier + Send + Sync;

pub struct CompositeNotifier {
    plugins: Vec<Box<DynNotifier>>,
}

impl CompositeNotifier {
    pub fn new(plugins: Vec<Box<DynNotifier>>) -> Self {
        CompositeNotifier { plugins }
    }
}

#[async_trait]
impl Notifier for CompositeNotifier {
    async fn notify(&self, title: &str, body: &str) {
        let futures = self.plugins.iter().map(|plugin| plugin.notify(title, body));
        future::join_all(futures).await;
    }

    async fn run(&self) {
        let runs = self.plugins.iter().map(|plugin| plugin.run());
        future::join_all(runs).await;
    }
}

pub use desktop::DesktopNotifier;
pub use xmpp::XMPPNotifier;
