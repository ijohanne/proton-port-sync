self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.proton-port-sync;
  package = self.packages.${pkgs.system}.default;
in
{
  options.services.proton-port-sync = {
    enable = lib.mkEnableOption "ProtonVPN NAT-PMP port sync for qBittorrent";

    gateway = lib.mkOption {
      type = lib.types.str;
      default = "10.2.0.1";
      description = "ProtonVPN WireGuard gateway IP";
    };

    qbtUrl = lib.mkOption {
      type = lib.types.str;
      default = "http://127.0.0.1:8080";
      description = "qBittorrent WebUI URL";
    };

    qbtUser = lib.mkOption {
      type = lib.types.str;
      default = "admin";
      description = "qBittorrent WebUI username";
    };

    qbtPasswordFile = lib.mkOption {
      type = lib.types.path;
      description = "Path to file containing qBittorrent WebUI password";
    };

    renewInterval = lib.mkOption {
      type = lib.types.int;
      default = 45;
      description = "Seconds between NAT-PMP renewals";
    };

    maxFailures = lib.mkOption {
      type = lib.types.int;
      default = 3;
      description = "Consecutive failures before restarting WireGuard";
    };

    wgUnit = lib.mkOption {
      type = lib.types.str;
      default = "wireguard-wg0.service";
      description = "WireGuard systemd unit to restart on failure";
    };

    metrics = {
      enable = lib.mkEnableOption "Prometheus metrics endpoint";

      address = lib.mkOption {
        type = lib.types.str;
        default = "127.0.0.1";
        description = "Address to bind the metrics HTTP server to";
      };

      port = lib.mkOption {
        type = lib.types.port;
        default = 9834;
        description = "Port for the metrics HTTP server";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.proton-port-sync = {
      description = "ProtonVPN NAT-PMP port sync for qBittorrent";
      after = [
        cfg.wgUnit
        "qbittorrent.service"
      ];
      wants = [ "qbittorrent.service" ];
      bindsTo = [ cfg.wgUnit ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        ExecStart =
          let
            metricsFlag = lib.optionalString cfg.metrics.enable
              "--metrics-addr ${cfg.metrics.address}:${toString cfg.metrics.port}";
            wrapper = pkgs.writeShellScript "proton-port-sync" ''
              exec ${lib.getBin package}/bin/proton-port-sync \
                --gateway ${cfg.gateway} \
                --qbt-url ${cfg.qbtUrl} \
                --qbt-user ${cfg.qbtUser} \
                --qbt-password-file "$CREDENTIALS_DIRECTORY/qbt-password" \
                --renew-interval ${toString cfg.renewInterval} \
                --max-failures ${toString cfg.maxFailures} \
                --wg-unit ${cfg.wgUnit} \
                ${metricsFlag}
            '';
          in
          toString wrapper;
        Restart = "on-failure";
        RestartSec = "5s";
        DynamicUser = false;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        NoNewPrivileges = true;
        LoadCredential = [ "qbt-password:${cfg.qbtPasswordFile}" ];
      };
    };
  };
}
