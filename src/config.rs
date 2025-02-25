use std::env;

use anyhow::Context as _;
use anyhow::ensure;
use url::Url;

pub struct MQTTCredentials {
    pub username: String,
    pub password: String,
}

pub struct MQTTConfig {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub topic: String,
    pub credentials: Option<MQTTCredentials>,
}

impl MQTTConfig {
    pub fn new(mqtt_url: &str, default_topic: &str) -> anyhow::Result<Self> {
        let url = Url::parse(mqtt_url).context("Invalid URL")?;
        let scheme = url.scheme().to_lowercase();

        ensure!(
            scheme == "mqtt" || scheme == "mqtts",
            "Invalid scheme: expected mqtt or mqtts, got {}",
            scheme
        );

        let host = url
            .host_str()
            .context("Invalid URL: missing host")?
            .to_owned();

        let port = url
            .port()
            .unwrap_or_else(|| if scheme == "mqtts" { 8883 } else { 1883 });

        let topic = {
            let path = url.path().trim_start_matches('/');

            if path.is_empty() {
                default_topic.to_owned()
            } else {
                path.to_owned()
            }
        };

        let has_username = !url.username().is_empty();

        let credentials = has_username.then_some({
            let password = if let Some(pass) = url.password() {
                log::warn!("It isn't safe to provide password in the command line!");
                pass.to_owned()
            } else if let Ok(pass) = env::var("MQTT_PASSWORD") {
                pass
            } else {
                log::info!("Note: Password can be provided with env MQTT_PASSWORD");
                rpassword::prompt_password("Password: ").context("Failed to read password")?
            };

            MQTTCredentials {
                username: url.username().to_owned(),
                password,
            }
        });

        Ok(Self {
            scheme,
            host,
            port,
            topic,
            credentials,
        })
    }
}
