use anyhow::Result;
use axum::{http, http::StatusCode, response::IntoResponse, routing::get, Router};
use prometheus::{
    Encoder, IntCounter, IntGauge, Registry, TextEncoder,
};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub struct Metrics {
    registry: Arc<Registry>,
    pub current_port: IntGauge,
    pub port_changes_total: IntCounter,
    pub last_change_timestamp_seconds: IntGauge,
    pub renewals_total: IntCounter,
    pub failures_total: IntCounter,
    pub wg_restarts_total: IntCounter,
}

impl Metrics {
    pub fn new() -> Result<Self> {
        let registry = Registry::new_custom(Some("proton_port_sync".into()), None)?;

        let current_port = IntGauge::new("current_port", "Currently mapped NAT-PMP port")?;
        let port_changes_total =
            IntCounter::new("port_changes_total", "Total number of port changes")?;
        let last_change_timestamp_seconds = IntGauge::new(
            "last_change_timestamp_seconds",
            "Unix timestamp of the last port change",
        )?;
        let renewals_total =
            IntCounter::new("renewals_total", "Total successful NAT-PMP renewals")?;
        let failures_total = IntCounter::new("failures_total", "Total NAT-PMP renewal failures")?;
        let wg_restarts_total =
            IntCounter::new("wg_restarts_total", "Total WireGuard restarts triggered")?;

        registry.register(Box::new(current_port.clone()))?;
        registry.register(Box::new(port_changes_total.clone()))?;
        registry.register(Box::new(last_change_timestamp_seconds.clone()))?;
        registry.register(Box::new(renewals_total.clone()))?;
        registry.register(Box::new(failures_total.clone()))?;
        registry.register(Box::new(wg_restarts_total.clone()))?;

        Ok(Self {
            registry: Arc::new(registry),
            current_port,
            port_changes_total,
            last_change_timestamp_seconds,
            renewals_total,
            failures_total,
            wg_restarts_total,
        })
    }

    pub fn record_port_change(&self, port: u16) {
        self.current_port.set(port as i64);
        self.port_changes_total.inc();
        self.last_change_timestamp_seconds.set(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        );
    }
}

async fn metrics_handler(metrics: Arc<Metrics>) -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = metrics.registry.gather();
    let mut buffer = Vec::new();
    match encoder.encode(&metric_families, &mut buffer) {
        Ok(()) => (
            StatusCode::OK,
            [(http::header::CONTENT_TYPE, encoder.format_type().to_owned())],
            buffer,
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(
                http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8".to_owned(),
            )],
            format!("Failed to encode metrics: {e}").into_bytes(),
        ),
    }
}

pub async fn serve(metrics: Metrics, addr: SocketAddr) -> Result<()> {
    let shared = Arc::new(metrics);
    let app = Router::new().route(
        "/metrics",
        get({
            let m = shared.clone();
            move || metrics_handler(m)
        }),
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "metrics server listening");
    axum::serve(listener, app).await?;
    Ok(())
}
