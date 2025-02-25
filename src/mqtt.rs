use core::error::Error as _;
use core::time::Duration;
use std::sync::Arc;

use anyhow::Context as _;
use rumqttc::{
    AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS, TlsConfiguration, Transport,
};

use crate::config::MQTTConfig;
use crate::notifier::DynNotifier;

pub struct MQTTNotificationClient {
    client: AsyncClient,
    eventloop: EventLoop,
    topic: String,
    notifier: Arc<DynNotifier>,
}

enum State {
    Polling,
    RetryDelay,
}

impl MQTTNotificationClient {
    pub fn new(config: &MQTTConfig, notifier: Arc<DynNotifier>) -> Self {
        let client_id = format!("mqtt-notify-rs-{}", rand::random::<u16>());
        let mut mqttoptions = MqttOptions::new(client_id, &config.host, config.port);

        if config.scheme == "mqtts" {
            mqttoptions.set_transport(Transport::Tls(TlsConfiguration::default()));
        }

        if let Some(ref credentials) = config.credentials {
            mqttoptions.set_credentials(&credentials.username, &credentials.password);
        }

        let (client, eventloop) = AsyncClient::new(mqttoptions, 10);

        Self {
            client,
            eventloop,
            topic: config.topic.clone(),
            notifier,
        }
    }

    pub async fn run(&mut self, shutdown: Arc<tokio::sync::Notify>) -> anyhow::Result<()> {
        let mut state = State::Polling;

        loop {
            tokio::select! {
                next_state = self.run_state(state) => state = next_state?,
                () = shutdown.notified() => break,
            }
        }

        self.client
            .disconnect()
            .await
            .context("Error during MQTT disconnect")
    }

    async fn run_state(&mut self, state: State) -> anyhow::Result<State> {
        match state {
            State::Polling => {
                match self.eventloop.poll().await {
                    Ok(Event::Incoming(packet)) => self.handle_packet(packet).await?,
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("MQTT error: {}", e.source().unwrap_or(&e));
                        return Ok(State::RetryDelay);
                    }
                }

                Ok(State::Polling)
            }
            State::RetryDelay => {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok(State::Polling)
            }
        }
    }

    async fn handle_packet(&self, packet: Packet) -> anyhow::Result<()> {
        match packet {
            Packet::ConnAck(_) => {
                log::info!("Connected to the MQTT broker");

                self.client
                    .subscribe(&self.topic, QoS::AtLeastOnce)
                    .await
                    .context("MQTT subscription error")?;
            }
            Packet::SubAck(_) => {
                log::info!("Listening for notifications on MQTT topic '{}'", self.topic);
            }
            Packet::Publish(publish) => {
                let payload = String::from_utf8_lossy(&publish.payload);
                let mut lines = payload.lines();

                if let Some(title) = lines.next() {
                    let body = lines.collect::<Vec<_>>().join("\n");
                    log::info!(">> {}: {}", title, body);
                    self.notifier.notify(title, &body).await;
                }
            }
            _ => {}
        }

        Ok(())
    }
}
