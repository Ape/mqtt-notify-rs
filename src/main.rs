#![warn(clippy::pedantic)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::str_to_string)]
#![warn(clippy::unused_trait_names)]

mod config;
mod mqtt;
mod notifier;

use std::io::Write as _;
use std::sync::Arc;

use clap::Parser as _;
use rustls::crypto;

use crate::config::MQTTConfig;
use crate::mqtt::MQTTNotificationClient;
use crate::notifier::{CompositeNotifier, DesktopNotifier, DynNotifier, XMPPNotifier};

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// MQTT URL (mqtt[s]://[user@]host[:port][/topic])
    mqtt_url: String,

    /// Enable desktop notifications
    #[arg(long)]
    desktop: bool,

    /// Enable XMPP notifications (can be specified multiple times)
    #[arg(long, value_name = "RECIPIENT")]
    xmpp: Vec<String>,

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

    let config = MQTTConfig::new(&args.mqtt_url, "notifications").unwrap_or_else(|e| {
        log::error!("{}", e);
        std::process::exit(1)
    });

    let mut notifiers: Vec<Box<DynNotifier>> = Vec::new();

    if args.desktop {
        notifiers.push(Box::new(DesktopNotifier::new()));
    }

    if !args.xmpp.is_empty() {
        match XMPPNotifier::from_credentials_file(&args.xmpp, &args.xmpp_credentials) {
            Ok(notifier) => notifiers.push(Box::new(notifier)),
            Err(e) => {
                log::error!("XMPP error: {}", e);
                std::process::exit(1);
            }
        }
    }

    if notifiers.is_empty() {
        log::warn!("No notifiers enabled");
    }

    let shutdown = Arc::new(tokio::sync::Notify::new());
    let composite: Arc<DynNotifier> = Arc::new(CompositeNotifier::new(notifiers));
    let mut mqtt = MQTTNotificationClient::new(&config, Arc::clone(&composite));

    tokio::join!(
        signal_handler(shutdown.clone()),
        mqtt.run(shutdown.clone()),
        composite.run(shutdown.clone()),
    );
}

async fn signal_handler(shutdown: Arc<tokio::sync::Notify>) {
    #[cfg(unix)]
    {
        use tokio::signal::unix;

        let mut sigint =
            unix::signal(unix::SignalKind::interrupt()).expect("Failed to install SIGINT handler");

        let mut sigterm =
            unix::signal(unix::SignalKind::terminate()).expect("Failed to install SIGTERM handler");

        tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL-C handler");
    }

    log::info!("Shutting down...");
    shutdown.notify_waiters();
}
