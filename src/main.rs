#![warn(clippy::pedantic)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::str_to_string)]
#![warn(clippy::unused_trait_names)]

mod config;
mod mqtt;
mod notifier;

use core::time::Duration;
use std::io::Write as _;
use std::sync::Arc;

use anyhow::Context as _;
use clap::Parser as _;
use rustls::crypto;
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};

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

    let result = Toplevel::new(async |toplevel: &mut SubsystemHandle| {
        let args = Args::parse();

        toplevel.start(SubsystemBuilder::new(
            "mqtt-notify",
            async |subsys: &mut SubsystemHandle| run(args, subsys).await,
        ));

        toplevel.start(SubsystemBuilder::new(
            "shutdown-logger",
            async |subsys: &mut SubsystemHandle| -> anyhow::Result<()> {
                subsys.on_shutdown_requested().await;
                log::info!("Shutting down...");
                Ok(())
            },
        ));
    })
    .catch_signals()
    .handle_shutdown_requests(Duration::from_secs(10))
    .await;

    if let Err(e) = result {
        log::error!("{e:#}");
    }
}

async fn run(args: Args, subsys: &SubsystemHandle) -> anyhow::Result<()> {
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

    subsys.start(SubsystemBuilder::new(
        "mqtt-client",
        async move |subsys: &mut SubsystemHandle| mqtt.run(subsys).await,
    ));

    let composite_runner = Arc::clone(&composite);
    subsys.start(SubsystemBuilder::new(
        "notifiers",
        async move |subsys: &mut SubsystemHandle| composite_runner.run(subsys).await,
    ));

    subsys.wait_for_children().await;

    Ok(())
}
