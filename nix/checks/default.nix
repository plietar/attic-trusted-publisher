{ self, moduleWithSystem, ... }: {
  imports = [ ./basic.nix ./reverse-proxy.nix ./asymmetric.nix ];

  perSystem = { pkgs, ... }: {
    packages.oidc-test-server = pkgs.writers.writePython3Bin "oidc-test-server"
      {
        libraries = ps: [ ps.bottle ps.jwcrypto ];
      } ./oidc-test-server.py;
  };

  flake.nixosModules.oidc-test-server = moduleWithSystem ({ self', ... }: { lib, config, ... }: {
    options.services.oidc-test-server = {
      enable = lib.mkEnableOption "oidc-test-server";
      port = lib.mkOption { type = lib.types.port; };
    };
    config = lib.mkIf config.services.oidc-test-server.enable {
      systemd.services.oidc-test-server = {
        wantedBy = [ "multi-user.target" ];
        serviceConfig.ExecStart = "${lib.getExe self'.packages.oidc-test-server} ${toString config.services.oidc-test-server.port}";
      };
    };
  });
}
