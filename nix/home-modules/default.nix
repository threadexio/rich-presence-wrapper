{
  ...
}:

{
  config,
  pkgs,
  lib,
  ...
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

      music-bridge = {
        enable = mkEnableOption "rich presence integration for music";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    xdg.configFile."rich-presence-wrapper/config.toml".source = "${configFile}";

    systemd.user.services = lib.optionalAttrs cfg.music-bridge.enable {
      "music-rich-presence" = {
        Unit = {
          Description = "Discord Rich Presence for playing media";
          After = [ "dbus.socket" ];
          Wants = [ "dbus.socket" ];
        };

        Service = {
          Type = "simple";
          ExecStart = "${lib.getExe cfg.package} music-bridge";
        };

        Install = {
          WantedBy = [ "graphical-session.target" ];
        };
      };
    };
  };
}
