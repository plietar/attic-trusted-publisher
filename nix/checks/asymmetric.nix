# This test uses asymmetric RSA keys for signing and verifying tokens.
#
# attic-trusted-publisher needs the private key to issue the tokens, but attic
# can run with just the public key.
{ self, moduleWithSystem, ... }: {
  perSystem = { pkgs, ... }: {
    checks.asymmetric = pkgs.testers.runNixOSTest {
      name = "asymmetric";

      nodes.server = moduleWithSystem ({ self', ... }: { pkgs, lib, ... }: {
        imports = [
          self.nixosModules.attic-trusted-publisher
          self.nixosModules.oidc-test-server
        ];
        networking.firewall.enable = false;

        systemd.services.generate-key-files = {
          wantedBy = [ "multi-user.target" ];
          before = [ "atticd.service" "attic-trusted-publisher.service" ];
          path = [ pkgs.openssl ];
          script = ''
            openssl genrsa -traditional -out /tmp/private.pem 4096
            openssl rsa -in /tmp/private.pem -pubout -out /tmp/public.pem
            echo "ATTIC_SERVER_TOKEN_RS256_PUBKEY_BASE64=$(base64 -w0 /tmp/public.pem)" > /var/lib/atticd.env
            echo "ATTIC_SERVER_TOKEN_RS256_SECRET_BASE64=$(base64 -w0 /tmp/private.pem)" > /var/lib/attic-trusted-publisher.env
          '';
          serviceConfig.Type = "oneshot";
        };

        services.atticd = {
          enable = true;
          settings.listen = "0.0.0.0:3000";
          environmentFile = "/var/lib/atticd.env";
        };
        services.attic-trusted-publisher = {
          enable = true;
          listen = "0.0.0.0:3001";
          settings.audience = "http://server:3000";
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
          environmentFile = "/var/lib/attic-trusted-publisher.env";
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
        server.wait_for_unit("default.target")
        client.wait_for_unit("default.target")

        server.wait_for_open_port(3000)
        server.wait_for_open_port(3001)
        server.wait_for_open_port(3002)

        client.succeed("""
          idtoken=$(curl -fLv http://server:3002/token --json '{ "aud": "http://server:3000", "repository": "foo" }')
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
