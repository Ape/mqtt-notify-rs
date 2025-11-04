pub mod desktop;
pub mod xmpp;

use async_trait::async_trait;
use futures::future;
use tokio_graceful_shutdown::SubsystemHandle;

#[async_trait]
pub trait Notifier {
    async fn notify(&self, title: &str, body: &str);

    async fn run(&self, _subsys: &SubsystemHandle) -> anyhow::Result<()> {
        Ok(())
    }
}

pub type DynNotifier = dyn Notifier + Send + Sync;

pub struct CompositeNotifier {
    notifiers: Vec<Box<DynNotifier>>,
}

impl CompositeNotifier {
    pub fn new(notifiers: Vec<Box<DynNotifier>>) -> Self {
        Self { notifiers }
    }
}

#[async_trait]
impl Notifier for CompositeNotifier {
    async fn notify(&self, title: &str, body: &str) {
        future::join_all(self.notifiers.iter().map(|x| x.notify(title, body))).await;
    }

    async fn run(&self, subsys: &SubsystemHandle) -> anyhow::Result<()> {
        future::try_join_all(self.notifiers.iter().map(|notifier| notifier.run(subsys))).await?;
        Ok(())
    }
}

pub use desktop::DesktopNotifier;
pub use xmpp::XMPPNotifier;
