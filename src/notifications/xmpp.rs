use std::fs;
use std::str::FromStr;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use xmpp::jid::BareJid;
use xmpp::{ClientBuilder, ClientType, Event};
use xmpp_parsers::message::MessageType;

use crate::notifications::NotificationPlugin;

pub struct XMPPNotificationPlugin {
    jid: BareJid,
    password: String,
    recipient: BareJid,
    sender: UnboundedSender<String>,
    receiver: Mutex<UnboundedReceiver<String>>,
}

impl XMPPNotificationPlugin {
    pub async fn new(jid: &str, password: &str, recipient: &str) -> Self {
        let jid = BareJid::from_str(jid).expect("Invalid JID");
        let recipient_jid = BareJid::from_str(recipient).expect("Invalid recipient JID");

        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            jid,
            password: password.to_string(),
            recipient: recipient_jid,
            sender,
            receiver: Mutex::new(receiver),
        }
    }

    pub async fn from_credentials_file(
        recipient: &str,
        filepath: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let path = shellexpand::tilde(filepath).to_string();
        let content = fs::read_to_string(&path)?;
        let mut parts = content.split_whitespace();

        let jid = parts
            .next()
            .ok_or_else(|| format!("Missing jid in {}", path))?;

        let password = parts
            .next()
            .ok_or_else(|| format!("Missing password in {}", path))?;

        Ok(Self::new(jid, password, recipient).await)
    }
}

#[async_trait]
impl NotificationPlugin for XMPPNotificationPlugin {
    async fn notify(&self, title: &str, body: &str) {
        let message = format!("{}\n{}", title, body);

        if let Err(e) = self.sender.send(message) {
            log::error!("Failed to send notification: {}", e);
        }
    }

    async fn run(&self) {
        let mut agent = ClientBuilder::new(self.jid.clone(), &self.password)
            .set_client(ClientType::Bot, "mqtt-notify-rs")
            .build();

        let mut receiver = self.receiver.lock().await;

        loop {
            tokio::select! {
                Some(msg) = receiver.recv() => {
                    agent.send_message(
                        self.recipient.clone().into(),
                        MessageType::Chat,
                        "",
                        &msg,
                    ).await;
                },
                Some(events) = agent.wait_for_events() => {
                    for event in events {
                        if let Event::Online = event {
                            log::info!("XMPP agent online as {}", agent.bound_jid().unwrap());
                        }
                    }
                }
            }
        }
    }
}
