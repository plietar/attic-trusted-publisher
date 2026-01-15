{ self, moduleWithSystem, ... }: {
  perSystem = { pkgs, ... }: {
    checks.reverse-proxy = pkgs.testers.runNixOSTest {
      name = "reverse-proxy";

      nodes.server = moduleWithSystem ({ self', ... }: { pkgs, ... }: {
        imports = [
          self.nixosModules.attic-trusted-publisher
          self.nixosModules.oidc-test-server
        ];
        networking.firewall.enable = false;
        services.atticd = {
          enable = true;
          settings.listen = "127.0.0.1:3000";
          settings.api-endpoint = "http://server";
          environmentFile = pkgs.runCommand "envfile" { } ''
            cat > $out <<EOF
            ATTIC_SERVER_TOKEN_HS256_SECRET_BASE64=$(echo -n "secret" | base64)
            EOF
          '';
        };
        services.attic-trusted-publisher = {
          enable = true;
          listen = "127.0.0.1:3001";
          settings.audience = "http://server";
          settings.policies = [{
            issuer = "http://server:3002";
            required_claims.repository = "foo";
            permissions."*" = {
              pull = true;
              push = true;
              create_cache = true;
              configure_cache = true;
            };
          }];
        };
        services.oidc-test-server = {
          enable = true;
          port = 3002;
        };
        services.nginx.enable = true;
        services.nginx.virtualHosts."server" = {
          locations."/" = {
            proxyPass = "http://127.0.0.1:3000";
          };
          locations."/_trusted-publisher" = {
            proxyPass = "http://127.0.0.1:3001";
          };
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
          idtoken=$(curl -fLv http://server:3002/token --json '{ "aud": "http://server", "repository": "foo" }')
          attictoken=$(attic-trusted-publisher login http://server "$idtoken")
          attic login default http://server "$attictoken"
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
