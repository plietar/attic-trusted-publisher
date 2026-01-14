{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/25.11";
    crane.url = "github:ipetkov/crane";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs: inputs.flake-parts.lib.mkFlake { inherit inputs; } ({ self, moduleWithSystem, ... }: {
    systems = [ "x86_64-linux" ];

    perSystem = { pkgs, self', ... }: {
      packages.default = self'.packages.attic-trusted-publisher;
      packages.attic-trusted-publisher =
        let craneLib = inputs.crane.mkLib pkgs;
        in craneLib.buildPackage {
          name = "attic-trusted-publisher";
          src = craneLib.cleanCargoSource ./.;
          meta.mainProgram = "attic-trusted-publisher";
        };

      devShells.attic-trusted-publisher = pkgs.mkShell {
        inputsFrom = [ self'.packages.attic-trusted-publisher ];
        nativeBuildInputs = [ pkgs.rustfmt ];
      };
    };

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
              listen = lib.mkOption {
                type = lib.types.str;
              };
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

          config = {
            systemd.services.attic-trusted-publisher = {
              wantedBy = [ "multi-user.target" ];
              serviceConfig = {
                ExecStart = "${lib.getExe self'.packages.attic-trusted-publisher} api --listen ${cfg.listen} --config ${configFile}";
                EnvironmentFile = lib.mkIf (cfg.environmentFile != null) cfg.environmentFile;
                DynamicUser = true;
                User = cfg.user;
                Group = cfg.group;
              };
            };
          };
        });
  });
}
