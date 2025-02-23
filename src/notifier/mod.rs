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
    notifiers: Vec<Box<DynNotifier>>,
}

impl CompositeNotifier {
    pub fn new(notifiers: Vec<Box<DynNotifier>>) -> Self {
        CompositeNotifier { notifiers }
    }
}

#[async_trait]
impl Notifier for CompositeNotifier {
    async fn notify(&self, title: &str, body: &str) {
        let futures = self
            .notifiers
            .iter()
            .map(|notifier| notifier.notify(title, body));
        future::join_all(futures).await;
    }

    async fn run(&self) {
        let futures = self.notifiers.iter().map(|notifier| notifier.run());
        future::join_all(futures).await;
    }
}

pub use desktop::DesktopNotifier;
pub use xmpp::XMPPNotifier;
