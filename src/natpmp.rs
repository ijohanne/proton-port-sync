use anyhow::{Context, Result};
use std::net::Ipv4Addr;

pub struct NatPmpClient {
    gateway: Ipv4Addr,
}

impl NatPmpClient {
    pub fn new(gateway: &str) -> Result<Self> {
        let gateway: Ipv4Addr = gateway
            .parse()
            .with_context(|| format!("invalid gateway IP: {gateway}"))?;
        Ok(Self { gateway })
    }

    pub fn request_mapping(&self, lifetime_secs: u32) -> Result<u16> {
        let mut client =
            natpmp::Natpmp::new_with(self.gateway).context("failed to create NAT-PMP client")?;

        client
            .send_port_mapping_request(natpmp::Protocol::TCP, 0, 0, lifetime_secs)
            .context("failed to send NAT-PMP request")?;

        let response = client
            .read_response_or_retry()
            .context("failed to read NAT-PMP response")?;

        match response {
            natpmp::Response::TCP(mapping) => Ok(mapping.public_port()),
            _ => anyhow::bail!("unexpected NAT-PMP response type"),
        }
    }
}
