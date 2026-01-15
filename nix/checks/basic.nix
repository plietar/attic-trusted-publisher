{ self, moduleWithSystem, ... }: {
  perSystem = { pkgs, ... }: {
    checks.basic = pkgs.testers.runNixOSTest {
      name = "basic";

      nodes.server = moduleWithSystem ({ self', ... }: { pkgs, ... }: {
        imports = [
          self.nixosModules.attic-trusted-publisher
          self.nixosModules.oidc-test-server
        ];
        networking.firewall.enable = false;
        services.atticd = {
          enable = true;
          settings.listen = "[::]:3000";
          environmentFile = pkgs.runCommand "envfile" { } ''
            cat > $out <<EOF
            ATTIC_SERVER_TOKEN_HS256_SECRET_BASE64=$(echo -n "secret" | base64)
            EOF
          '';
        };
        services.attic-trusted-publisher = {
          enable = true;
          settings = {
            listen = "[::]:3001";
            audience = "http://server:3001";
            policies = [{
              issuer = "http://server:3002";
              duration = "1h";
              required_claims.repository = "foo";
              permissions."*" = {
                pull = true;
                push = true;
                create_cache = true;
                configure_cache = true;
              };
            }];
          };
        };
        services.oidc-test-server = {
          enable = true;
          port = 3002;
        };
      });

      nodes.client = moduleWithSystem ({ self', ... }: { pkgs, ... }: {
        environment.systemPackages = [
          self'.packages.attic-trusted-publisher
          pkgs.attic-client
          pkgs.openssl
        ];
      });

      testScript = ''
        start_all()
        server.wait_for_unit("default.target")
        client.wait_for_unit("default.target")

        server.wait_for_open_port(3000)
        server.wait_for_open_port(3001)
        server.wait_for_open_port(3002)

        client.succeed("""
          idtoken=$(curl -sfL http://server:3002/token --json '{ "aud": "http://server:3001", "repository": "foo" }')
          attictoken=$(attic-trusted-publisher login http://server:3001 "$idtoken")
          attic login default http://server:3000 "$attictoken"
        """)

        client.succeed("attic cache create mycache")
        client.succeed("""
          openssl rand -base64 > data
          path=$(nix store add --mode flat --extra-experimental-features nix-command data)
          attic push mycache "$path"
        """)
      '';
    };
  };
}
