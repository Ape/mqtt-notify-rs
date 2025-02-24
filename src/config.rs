use std::env;
use std::io;

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

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Invalid MQTT URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Invalid MQTT scheme: expected mqtt or mqtts")]
    InvalidScheme,

    #[error("Invalid MQTT URL: missing host")]
    MissingHost,

    #[error("Failed to read password: {0}")]
    PasswordReadError(#[from] io::Error),
}

impl MQTTConfig {
    pub fn new(mqtt_url: &str, default_topic: &str) -> Result<Self, ConfigError> {
        let url = Url::parse(mqtt_url)?;
        let scheme = url.scheme().to_lowercase();

        if scheme != "mqtt" && scheme != "mqtts" {
            return Err(ConfigError::InvalidScheme);
        }

        let host = url.host_str().ok_or(ConfigError::MissingHost)?.to_owned();

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
                rpassword::prompt_password("Password: ")?
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
