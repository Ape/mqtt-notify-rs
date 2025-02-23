mod config;
mod mqtt;
mod notifications;

use std::io::Write;
use std::sync::Arc;

use clap::Parser;
use rustls::crypto;

use crate::config::MQTTConfig;
use crate::mqtt::MQTTNotificationClient;
use crate::notifications::{
    CompositeNotificationPlugin, DesktopNotificationPlugin, DynNotificationPlugin,
    XMPPNotificationPlugin,
};

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// MQTT URL (mqtt[s]://[user@]host[:port][/topic])
    mqtt_url: String,

    /// Enable desktop notifications
    #[arg(long)]
    desktop: bool,

    /// Enable XMPP notifications
    #[arg(long, value_name = "RECIPIENT")]
    xmpp: Option<String>,

    /// Path to the XMPP credentials file
    #[arg(long, value_name = "FILE", default_value = "~/.sendxmpprc")]
    xmpp_credentials: String,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,xmpp::disco=warn"),
    )
    .format(|buf, record| {
        if record.level() == log::Level::Info {
            writeln!(buf, "{}", record.args())
        } else {
            writeln!(buf, "[{}] {}", record.level(), record.args())
        }
    })
    .init();

    crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install crypto provider");

    let args = Args::parse();

    let config = match MQTTConfig::new(&args.mqtt_url, "notifications") {
        Ok(cfg) => cfg,
        Err(e) => {
            log::error!("{}", e);
            std::process::exit(1);
        }
    };

    let mut plugins: Vec<Box<DynNotificationPlugin>> = Vec::new();

    if args.desktop {
        plugins.push(Box::new(DesktopNotificationPlugin::new()));
    }

    if let Some(recipient) = args.xmpp {
        match XMPPNotificationPlugin::from_credentials_file(&recipient, &args.xmpp_credentials)
            .await
        {
            Ok(plugin) => plugins.push(Box::new(plugin)),
            Err(e) => {
                log::error!(
                    "Error loading XMPP credentials from '{}': {}",
                    args.xmpp_credentials,
                    e
                );
                std::process::exit(1);
            }
        }
    }

    if plugins.is_empty() {
        log::warn!("No notification plugins enabled");
    }

    let composite: Arc<DynNotificationPlugin> = Arc::new(CompositeNotificationPlugin::new(plugins));
    let mut mqtt = MQTTNotificationClient::new(&config, Arc::clone(&composite));

    tokio::join!(mqtt.run(), composite.run());
}
