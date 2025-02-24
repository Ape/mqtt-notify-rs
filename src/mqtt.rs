use core::error::Error as _;
use core::time::Duration;
use std::sync::Arc;

use rumqttc::{
    AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS, TlsConfiguration, Transport,
};
use tokio::time;

use crate::config::MQTTConfig;
use crate::notifier::DynNotifier;

pub struct MQTTNotificationClient {
    client: AsyncClient,
    eventloop: EventLoop,
    topic: String,
    notifier: Arc<DynNotifier>,
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

    pub async fn run(&mut self, shutdown: Arc<tokio::sync::Notify>) {
        if let Err(e) = self.client.subscribe(&self.topic, QoS::AtLeastOnce).await {
            log::error!("MQTT subscription error: {:?}", e);
            return;
        }

        loop {
            tokio::select! {
                () = shutdown.notified() => {
                    if let Err(e) = self.client.disconnect().await {
                        log::error!("Error during MQTT disconnect: {}", e);
                    }

                    return;
                }
                result = self.eventloop.poll() => {
                    match result {
                        Ok(Event::Incoming(packet)) => match packet {
                            Packet::ConnAck(_) => {
                                log::info!("Connected to the MQTT broker");
                            }
                            Packet::SubAck(_) => {
                                log::info!("Listening for notifications on MQTT topic '{}'", self.topic);
                            }
                            Packet::Publish(publish) => {
                                let payload = String::from_utf8_lossy(&publish.payload);
                                let mut lines = payload.lines();

                                if let Some(title) = lines.next() {
                                    let body = lines.collect::<Vec<&str>>().join("\n");
                                    log::info!(">> {}: {}", title, body);
                                    self.notifier.notify(title, &body).await;
                                }
                            }
                            _ => {}
                        }
                        Ok(_) => {}
                        Err(e) => {
                            log::error!("MQTT error: {}", e.source().unwrap_or(&e));
                            time::sleep(Duration::from_secs(2)).await;
                        }
                    }
                }
            }
        }
    }
}
