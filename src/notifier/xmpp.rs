use core::error::Error;
use core::str::FromStr as _;
use std::fs;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use xmpp::jid::BareJid;
use xmpp::{ClientBuilder, ClientType, Event};
use xmpp_parsers::message::MessageType;

use crate::notifier::Notifier;

pub struct XMPPNotifier {
    jid: BareJid,
    password: String,
    recipients: Vec<BareJid>,
    sender: UnboundedSender<String>,
    receiver: Mutex<UnboundedReceiver<String>>,
}

impl XMPPNotifier {
    pub fn new(jid: &str, password: &str, recipients: &[String]) -> Self {
        let jid = BareJid::from_str(jid).expect("Invalid JID");
        let recipient_jids = recipients
            .iter()
            .map(|x| BareJid::from_str(x).expect("Invalid recipient JID"))
            .collect();

        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            jid,
            password: password.to_string(),
            recipients: recipient_jids,
            sender,
            receiver: Mutex::new(receiver),
        }
    }

    pub fn from_credentials_file(
        recipients: &[String],
        filepath: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let path = shellexpand::tilde(filepath).to_string();
        let content = fs::read_to_string(&path)?;
        let mut parts = content.split_whitespace();

        let jid = parts
            .next()
            .ok_or_else(|| format!("Missing jid in {path}"))?;

        let password = parts
            .next()
            .ok_or_else(|| format!("Missing password in {path}"))?;

        Ok(Self::new(jid, password, recipients))
    }
}

#[async_trait]
impl Notifier for XMPPNotifier {
    async fn notify(&self, title: &str, body: &str) {
        let message = format!("{title}\n{body}");

        if let Err(e) = self.sender.send(message) {
            log::error!("Failed to send notification: {}", e);
        }
    }

    async fn run(&self) {
        let mut agent = ClientBuilder::new(self.jid.clone(), &self.password)
            .set_client(ClientType::Bot, "mqtt-notify-rs")
            .build();

        loop {
            tokio::select! {
                msg = async { self.receiver.lock().await.recv().await } => {
                    if let Some(msg) = msg {
                        for recipient in &self.recipients {
                            agent.send_message(
                                recipient.clone().into(),
                                MessageType::Chat,
                                "",
                                &msg,
                            ).await;
                        }
                    }
                },
                events = agent.wait_for_events() => {
                    if let Some(events) = events {
                        for event in events {
                            if matches!(event, Event::Online) {
                                log::info!("XMPP agent online as {}", agent.bound_jid().unwrap());
                            }
                        }
                    }
                }
            }
        }
    }
}
