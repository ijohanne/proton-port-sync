mod config;
mod metrics;
mod natpmp;
mod qbittorrent;

use anyhow::{Context, Result};
use clap::Parser;
use std::process::Command;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = config::Config::parse();

    let password = std::fs::read_to_string(&cfg.qbt_password_file)
        .with_context(|| format!("reading password from {:?}", cfg.qbt_password_file))?
        .trim()
        .to_string();

    let natpmp_client = natpmp::NatPmpClient::new(&cfg.gateway, &cfg.bind_address)?;
    let mut qbt = qbittorrent::QbtClient::new(&cfg.qbt_url, &cfg.qbt_user, &password);

    let prom = if let Some(ref addr_str) = cfg.metrics_addr {
        let m = metrics::Metrics::new()?;
        let addr: std::net::SocketAddr = addr_str
            .parse()
            .with_context(|| format!("invalid metrics address: {addr_str}"))?;
        let serve_metrics = m.clone();
        tokio::spawn(async move {
            if let Err(e) = metrics::serve(serve_metrics, addr).await {
                error!(?e, "metrics server failed");
            }
        });
        Some(m)
    } else {
        None
    };

    let mut current_port: Option<u16> = None;
    let mut fail_count: u32 = 0;

    info!(gateway = %cfg.gateway, bind_address = %cfg.bind_address, "starting port sync loop");

    loop {
        match natpmp_client.request_mapping(60) {
            Ok(port) => {
                fail_count = 0;
                if let Some(ref m) = prom {
                    m.renewals_total.inc();
                }

                if current_port != Some(port) {
                    info!(port, "NAT-PMP port mapped");

                    match qbt.set_listen_port(port).await {
                        Ok(()) => {
                            info!(port, "qBittorrent listening port updated");
                            current_port = Some(port);
                            if let Some(ref m) = prom {
                                m.record_port_change(port);
                            }
                        }
                        Err(e) => {
                            error!(?e, "failed to update qBittorrent port");
                        }
                    }
                }
            }
            Err(e) => {
                fail_count += 1;
                if let Some(ref m) = prom {
                    m.failures_total.inc();
                }
                warn!(?e, fail_count, "NAT-PMP renewal failed");

                if fail_count >= cfg.max_failures {
                    warn!(unit = %cfg.wg_unit, "too many failures, restarting WireGuard");

                    let status = Command::new("systemctl")
                        .args(["restart", &cfg.wg_unit])
                        .status();

                    match status {
                        Ok(s) if s.success() => {
                            info!("WireGuard restarted successfully");
                            if let Some(ref m) = prom {
                                m.wg_restarts_total.inc();
                            }
                        }
                        Ok(s) => error!(?s, "WireGuard restart exited non-zero"),
                        Err(e) => error!(?e, "failed to run systemctl"),
                    }

                    fail_count = 0;
                    current_port = None;
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }

                sleep(Duration::from_secs(15)).await;
                continue;
            }
        }

        sleep(Duration::from_secs(cfg.renew_interval)).await;
    }
}
