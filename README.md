# proton-port-sync

Automatically syncs ProtonVPN's NAT-PMP forwarded port to qBittorrent's listening port.

ProtonVPN assigns dynamic ports via NAT-PMP that change periodically. This service continuously renews the port mapping and updates qBittorrent so torrents remain connectable.

## How it works

1. Requests both UDP and TCP port mappings from the ProtonVPN gateway via NAT-PMP (RFC 6886)
2. When the mapped port changes, updates qBittorrent's listening port through its WebUI API
3. If NAT-PMP renewals fail repeatedly, restarts the WireGuard interface to recover

### NAT-PMP implementation notes

This project implements the NAT-PMP protocol directly rather than using the `natpmp` Rust crate. Two reasons:

- **Source address binding**: The `natpmp` crate binds its UDP socket to `0.0.0.0:0`. When using policy routing (e.g., table 51820 matching `from 10.2.0.2`), response packets from the gateway aren't routed back to the socket because the kernel doesn't associate the unbound socket with the VPN interface. The `--bind-address` option binds the socket to the WireGuard interface IP directly, ensuring responses traverse the correct routing table.

- **Internal port must be non-zero**: Per RFC 6886, an internal port of 0 in a mapping request means "delete all mappings for this protocol". ProtonVPN expects internal port 1 (the actual value doesn't matter since ProtonVPN assigns ports server-side, but it must be non-zero).

## Installation (NixOS)

Add the flake to your inputs and enable the module:

```nix
{
  inputs.proton-port-sync.url = "github:youruser/proton-port-sync";

  outputs = { self, nixpkgs, proton-port-sync, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      modules = [
        proton-port-sync.nixosModules.default
        {
          services.proton-port-sync = {
            enable = true;
            qbtPasswordFile = "/run/secrets/qbt-password";

            # Optional: enable Prometheus metrics
            metrics = {
              enable = true;
              address = "127.0.0.1";
              port = 9834;
            };
          };
        }
      ];
    };
  };
}
```

## Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `enable` | `false` | Enable the service |
| `gateway` | `10.2.0.1` | ProtonVPN WireGuard gateway IP |
| `bindAddress` | `10.2.0.2` | Local IP to bind NAT-PMP socket to (must be on the VPN interface) |
| `qbtUrl` | `http://127.0.0.1:8080` | qBittorrent WebUI URL |
| `qbtUser` | `admin` | qBittorrent WebUI username |
| `qbtPasswordFile` | *(required)* | Path to file containing qBittorrent password |
| `renewInterval` | `45` | Seconds between NAT-PMP renewals |
| `maxFailures` | `3` | Consecutive failures before restarting WireGuard |
| `wgUnit` | `wireguard-wg0.service` | WireGuard systemd unit to restart |
| `metrics.enable` | `false` | Enable Prometheus metrics endpoint |
| `metrics.address` | `127.0.0.1` | Metrics server bind address |
| `metrics.port` | `9834` | Metrics server port |

## CLI usage

```
proton-port-sync \
  --gateway 10.2.0.1 \
  --bind-address 10.2.0.2 \
  --qbt-url http://127.0.0.1:8080 \
  --qbt-user admin \
  --qbt-password-file /path/to/password \
  --metrics-addr 127.0.0.1:9834
```

All flags can also be set via environment variables: `NATPMP_GATEWAY`, `NATPMP_BIND_ADDRESS`, `QBT_URL`, `QBT_USER`, `QBT_PASSWORD_FILE`, `RENEW_INTERVAL`, `MAX_FAILURES`, `WG_UNIT`, `METRICS_ADDR`.

## Prometheus metrics

When `--metrics-addr` is provided, an HTTP endpoint is exposed at `/metrics` with:

| Metric | Type | Description |
|--------|------|-------------|
| `proton_port_sync_current_port` | Gauge | Currently mapped NAT-PMP port |
| `proton_port_sync_port_changes_total` | Counter | Total number of port changes |
| `proton_port_sync_last_change_timestamp_seconds` | Gauge | Unix timestamp of the last port change |
| `proton_port_sync_renewals_total` | Counter | Total successful NAT-PMP renewals |
| `proton_port_sync_failures_total` | Counter | Total NAT-PMP renewal failures |
| `proton_port_sync_wg_restarts_total` | Counter | Total WireGuard restarts triggered |

## Grafana dashboard

A sample Grafana dashboard is provided in [`grafana-dashboard.json`](./grafana-dashboard.json). Import it into Grafana and set your Prometheus data source.

## License

AGPL-3.0-only
