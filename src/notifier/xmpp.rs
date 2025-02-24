use core::error::Error;
use core::str::FromStr as _;
use std::fs;
use std::sync::Arc;

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
    pub fn new(jid: &str, password: &str, recipients: &[String]) -> Result<Self, Box<dyn Error>> {
        let jid =
            BareJid::from_str(jid).map_err(|e| format!("Failed to parse JID '{jid}': {e}"))?;

        let recipient_jids: Vec<BareJid> = recipients
            .iter()
            .map(|x| {
                BareJid::from_str(x)
                    .map_err(|e| format!("Failed to parse recipient JID '{x}': {e}"))
            })
            .collect::<Result<_, _>>()?;

        let (sender, receiver) = mpsc::unbounded_channel();

        Ok(Self {
            jid,
            password: password.to_owned(),
            recipients: recipient_jids,
            sender,
            receiver: Mutex::new(receiver),
        })
    }

    pub fn from_credentials_file(
        recipients: &[String],
        filepath: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let path = shellexpand::tilde(filepath).to_string();
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read credentials file '{path}': {e}"))?;
        let mut parts = content.split_whitespace();

        let jid = parts
            .next()
            .ok_or_else(|| format!("Missing jid in '{path}'"))?;

        let password = parts
            .next()
            .ok_or_else(|| format!("Missing password in '{path}'"))?;

        Self::new(jid, password, recipients)
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

    async fn run(&self, shutdown: Arc<tokio::sync::Notify>) {
        let mut agent = ClientBuilder::new(self.jid.clone(), &self.password)
            .set_client(ClientType::Bot, "mqtt-notify-rs")
            .build();

        loop {
            tokio::select! {
                () = shutdown.notified() => {
                    if let Err(e) = agent.disconnect().await {
                        log::error!("Error during XMPP disconnect: {}", e);
                    }

                    return;
                }
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
                }
                events = agent.wait_for_events() => {
                    if let Some(events) = events {
                        for event in events {
                            if matches!(event, Event::Online) {
                                if let Some(bound_jid) = agent.bound_jid() {
                                    log::info!("XMPP agent online as {}", bound_jid);
                                } else {
                                    log::warn!("XMPP agent online without JID");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
