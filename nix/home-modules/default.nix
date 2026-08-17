{ ...
}:

{ config
, pkgs
, lib
, ...
}:

let
  cfg = config.programs.rich-presence-wrapper;

  configFormat = pkgs.formats.toml { };
  configFile = configFormat.generate "rich-presence-wrapper-config.toml" cfg.settings;
in

{
  options = with lib; {
    programs.rich-presence-wrapper = {
      enable = mkEnableOption "rich-presence-wrapper";

      package = mkOption {
        description = ''
          The rich-presence-wrapper package to use.
        '';
        type = types.package;
        default = pkgs.rich-presence-wrapper;
      };

      settings = mkOption {
        description = ''
          Settings for rich-presence-wrapper.
        '';

        type = types.submodule {
          freeformType = configFormat.type;
          options = { };
        };

        default = { };
        example = {
          imports = [ "./other.toml" ];

          helix.path = "${pkgs.helix}/bin/hx";

          zed-editor = {
            path = "${pkgs.zed-editor}/bin/zeditor";
            client-id = "122133";
          };
        };
      };

      mpris-bridge = {
        enable = mkEnableOption "rich presence integration for MPRIS";

        player = mkOption {
          description = ''
            Bridge streams from only this player.

            Will be passed to `playerctl`'s `--player` argument.
          '';

          type = types.nullOr types.str;
          default = null;
          example = "vlc";
        };
      };
    };
  };

  config = lib.mkIf cfg.enable {
    xdg.configFile."rich-presence-wrapper/config.toml".source = "${configFile}";

    systemd.user.services = lib.optionalAttrs cfg.mpris-bridge.enable {
      "mpris-rich-presence" = {
        Unit = {
          Description = "Bridge MPRIS and Rich Presence";
          After = [ "dbus.socket" ];
          Wants = [ "dbus.socket" ];
        };

        Service = {
          Type = "simple";
          ExecStart =
            let
              player =
                if cfg.mpris-bridge.player != null
                then "--player '${cfg.mpris-bridge.player}'"
                else "";
            in
            "${lib.getExe cfg.package} mpris-bridge ${player}";
        };

        Install = {
          WantedBy = [ "graphical-session.target" ];
        };
      };
    };
  };
}
