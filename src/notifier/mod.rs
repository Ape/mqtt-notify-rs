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
        future::join_all(self.notifiers.iter().map(|x| x.notify(title, body))).await;
    }

    async fn run(&self) {
        future::join_all(self.notifiers.iter().map(|x| x.run())).await;
    }
}

pub use desktop::DesktopNotifier;
pub use xmpp::XMPPNotifier;
