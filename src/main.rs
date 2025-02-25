#![warn(clippy::pedantic)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::str_to_string)]
#![warn(clippy::unused_trait_names)]

mod config;
mod mqtt;
mod notifier;

use std::io::Write as _;
use std::sync::Arc;

use anyhow::Context as _;
use anyhow::bail;
use clap::Parser as _;
use futures::stream::StreamExt as _;
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

    let shutdown = Arc::new(tokio::sync::Notify::new());

    if let Err(e) = tokio::select! {
        result = run(Args::parse(), shutdown.clone()) => result,
        result = signal_handler(shutdown.clone()) => result,
    } {
        log::error!("{e:#}");
    };
}

async fn run(args: Args, shutdown: Arc<tokio::sync::Notify>) -> anyhow::Result<()> {
    let config = MQTTConfig::new(&args.mqtt_url, "notifications").context("MQTT config error")?;

    let mut notifiers: Vec<Box<DynNotifier>> = Vec::new();

    if args.desktop {
        notifiers.push(Box::new(DesktopNotifier::new()));
    }

    if !args.xmpp.is_empty() {
        let notifier = XMPPNotifier::from_credentials_file(&args.xmpp, &args.xmpp_credentials)
            .context("XMPP error")?;
        notifiers.push(Box::new(notifier));
    }

    if notifiers.is_empty() {
        log::warn!("No notifiers enabled");
    }

    let composite: Arc<DynNotifier> = Arc::new(CompositeNotifier::new(notifiers));
    let mut mqtt = MQTTNotificationClient::new(&config, Arc::clone(&composite));

    tokio::try_join!(mqtt.run(shutdown.clone()), composite.run(shutdown.clone()))?;
    Ok(())
}

async fn signal_handler(shutdown: Arc<tokio::sync::Notify>) -> anyhow::Result<()> {
    let mut signals = signal_hook_tokio::Signals::new([
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGTERM,
    ])
    .context("Failed to install signal handlers")?;

    signals.next().await;
    log::info!("Shutting down...");
    shutdown.notify_waiters();

    while let Some(signal) = signals.next().await {
        if signal == signal_hook::consts::SIGINT {
            bail!("Didn't have time to gracefully disconnect and cleanup");
        }
    }

    Ok(())
}
