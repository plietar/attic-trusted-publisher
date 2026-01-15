{ self, moduleWithSystem, ... }: {
  flake.nixosModules.default = self.nixosModules.attic-trusted-publisher;
  flake.nixosModules.attic-trusted-publisher = moduleWithSystem
    ({ self', ... }: { pkgs, config, lib, ... }:
      let
        format = pkgs.formats.toml { };
        cfg = config.services.attic-trusted-publisher;
        configFile = format.generate "config.toml" cfg.settings;
      in
      {
        options = {
          services.attic-trusted-publisher = {
            enable = lib.mkEnableOption "attic-trusted-publisher";
            settings = lib.mkOption {
              type = format.type;
              default = { };
            };
            environmentFile = lib.mkOption {
              type = lib.types.nullOr lib.types.path;
              default = config.services.atticd.environmentFile;
            };
            user = lib.mkOption {
              type = lib.types.str;
              default = config.services.atticd.user;
            };
            group = lib.mkOption {
              type = lib.types.str;
              default = config.services.atticd.group;
            };
          };
        };

        config = lib.mkIf cfg.enable {
          systemd.services.attic-trusted-publisher = {
            wantedBy = [ "multi-user.target" ];
            serviceConfig = {
              ExecStart = "${lib.getExe self'.packages.attic-trusted-publisher} api --config ${configFile}";
              EnvironmentFile = lib.mkIf (cfg.environmentFile != null) cfg.environmentFile;
              DynamicUser = true;
              User = cfg.user;
              Group = cfg.group;
            };
          };
        };
      });
}
