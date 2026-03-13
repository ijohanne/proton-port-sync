use anyhow::{Context, Result};
use std::net::{Ipv4Addr, UdpSocket};
use std::time::Duration;
use tracing::debug;

const NATPMP_PORT: u16 = 5351;
const NATPMP_VERSION: u8 = 0;
const NATPMP_OP_TCP_MAP: u8 = 2;
const NATPMP_RESULT_SUCCESS: u16 = 0;

pub struct NatPmpClient {
    gateway: Ipv4Addr,
    bind_addr: Ipv4Addr,
}

impl NatPmpClient {
    pub fn new(gateway: &str, bind_addr: &str) -> Result<Self> {
        let gateway: Ipv4Addr = gateway
            .parse()
            .with_context(|| format!("invalid gateway IP: {gateway}"))?;
        let bind_addr: Ipv4Addr = bind_addr
            .parse()
            .with_context(|| format!("invalid bind address: {bind_addr}"))?;
        Ok(Self { gateway, bind_addr })
    }

    pub fn request_mapping(&self, lifetime_secs: u32) -> Result<u16> {
        let sock = UdpSocket::bind((self.bind_addr, 0))
            .with_context(|| format!("failed to bind UDP socket to {}", self.bind_addr))?;
        sock.connect((self.gateway, NATPMP_PORT))
            .with_context(|| format!("failed to connect to {}:{}", self.gateway, NATPMP_PORT))?;

        // NAT-PMP mapping request: version(1) + opcode(1) + reserved(2) + internal_port(2) + external_port(2) + lifetime(4) = 12 bytes
        let mut request = [0u8; 12];
        request[0] = NATPMP_VERSION;
        request[1] = NATPMP_OP_TCP_MAP;
        // reserved bytes [2..4] = 0
        // internal_port [4..6] = 0 (let gateway choose)
        // external_port [6..8] = 0 (let gateway choose)
        request[8..12].copy_from_slice(&lifetime_secs.to_be_bytes());

        // RFC 6886: retry with exponential backoff, initial 250ms, doubling up to 9 times
        let mut timeout_ms = 250u64;
        for attempt in 0..9 {
            sock.set_read_timeout(Some(Duration::from_millis(timeout_ms)))
                .context("failed to set socket timeout")?;

            sock.send(&request).context("failed to send NAT-PMP request")?;
            debug!(attempt, timeout_ms, "sent NAT-PMP request");

            let mut buf = [0u8; 16];
            match sock.recv(&mut buf) {
                Ok(n) if n >= 16 => {
                    return Self::parse_response(&buf);
                }
                Ok(n) => {
                    anyhow::bail!("NAT-PMP response too short: {n} bytes");
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    debug!(attempt, "NAT-PMP response timeout, retrying");
                    timeout_ms *= 2;
                }
                Err(e) => {
                    return Err(e).context("failed to receive NAT-PMP response");
                }
            }
        }

        anyhow::bail!("NAT-PMP request timed out after 9 attempts")
    }

    fn parse_response(buf: &[u8; 16]) -> Result<u16> {
        // Response: version(1) + opcode(1) + result(2) + epoch(4) + internal_port(2) + external_port(2) + lifetime(4)
        let result_code = u16::from_be_bytes([buf[2], buf[3]]);
        if result_code != NATPMP_RESULT_SUCCESS {
            anyhow::bail!("NAT-PMP error: result code {result_code}");
        }

        let opcode = buf[1];
        if opcode != 128 + NATPMP_OP_TCP_MAP {
            anyhow::bail!("unexpected NAT-PMP response opcode: {opcode}");
        }

        let external_port = u16::from_be_bytes([buf[10], buf[11]]);
        Ok(external_port)
    }
}
