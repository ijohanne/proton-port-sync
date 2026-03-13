use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "proton-port-sync",
    about = "Sync ProtonVPN NAT-PMP port to qBittorrent"
)]
pub struct Config {
    #[arg(long, env = "NATPMP_GATEWAY", default_value = "10.2.0.1")]
    pub gateway: String,

    #[arg(long, env = "QBT_URL", default_value = "http://127.0.0.1:8080")]
    pub qbt_url: String,

    #[arg(long, env = "QBT_USER", default_value = "admin")]
    pub qbt_user: String,

    #[arg(long, env = "QBT_PASSWORD_FILE")]
    pub qbt_password_file: PathBuf,

    #[arg(long, env = "RENEW_INTERVAL", default_value = "45")]
    pub renew_interval: u64,

    #[arg(long, env = "MAX_FAILURES", default_value = "3")]
    pub max_failures: u32,

    #[arg(long, env = "WG_UNIT", default_value = "wireguard-wg0.service")]
    pub wg_unit: String,
}
