# Attic Trusted Publisher

This flake provides a companion service for
[attic](https://github.com/zhaofengli/attic), adding support for authentication
with OIDC ID tokens.

It implements a scheme similar to that provided by various online package
repositories (including PyPI, crates.io and more), commonly known as "Trusted
Publishers".

It allows a batch job (eg. a GitHub Actions workflow) to log in to attic
without having to manually provision secrets. Instead, the job obtains an ID
token from its platform and makes a request to attic-trusted-publisher to
exchange it for an attic token. The flow for obtaining the ID token depends on
the provider.

attic-trusted-publisher relies on having access to the same signing secret used
by attic, ie. the `token-rs256-secret-base64` or `token-hs256-secret-base64`
configuration value. Thanks to attic's stateless authentication, the two
services do not need to communicate.

# Usage
## Server configuration

Add this flake as an input and import its module into your machine
configuration.

```nix
{
  imports = [ inputs.attic-trusted-publisher.nixosModules.default ];
  services.atticd = {
    enable = true;
    environmentFile = "/secrets/attic-env"; # Contains the signing secret, see attic documentation
  };
  services.attic-trusted-publisher = {
    enable = true;
    settings = {
      audience = "http://atp.example.com"; # Must match the aud claim of ID tokens
      policies = []; # See below
    };
  };
}
```

By default, the attic-trusted-publisher module will reuse the value of the
`environmentFile` option of the attic module. This can be overriden, if the two
services run on separate machines for example.

## Writing policies

attic-trusted-publisher uses a list of policies to decide which ID tokens to
accept and what permissions to grant.

Policies are matched based on an issuer (which must match the `iss` claim from
the ID token) and a list of additional required claims. To avoid obvious
misconfigurations, at least one required claim must be specified.

If an ID token matches multiple policies, only the first one is used.

Refer to your OIDC provider's documentation for its list of provided claims:
- [GitHub Actions](https://docs.github.com/en/actions/concepts/security/openid-connect#understanding-the-oidc-token)
- [GitLab CI/CD](https://docs.gitlab.com/ci/secrets/id_token_authentication/#token-payload)

Below is an example of a configuration for a GitHub repository:

```nix
{
  services.attic-trusted-publisher.settings.policies = [{
    issuer = "https://token.actions.githubusercontent.com";
    required_claims = {
      repository = "owner/repo";
      repository_owner_id = 1234; # Needed to prevent account resurrection attacks
    };
    duration = "24h";
    allow_extending_token_lifespan = true;
    permissions."mycache" = {
      pull = true;
      push = true;
    };
  }];
```

More expressive policy definitions could be supported in the future. For
instance, once may want to use a claim value in the name of the cache, giving
each repository its own independent cache. 

## Client usage

The following command obtains an OIDC token based on its environment, exchanges
it for an attic token and prints the result to the console.

```
nix run github:plietar/attic-trusted-publisher login http://atp.example.com
```

Most commonly you will want to pass the result to the `attic login` command:
```
attic login myserver http://attic.example.com $(nix run github:plietar/attic-trusted-publisher login http://atp.example.com)
```

It supports running in GitHub Actions using the `ACTIONS_ID_TOKEN_REQUEST_URL`
and `ACTIONS_ID_TOKEN_REQUEST_TOKEN` environment variables. It will also
attempt to use the `ATTIC_TRUSTED_PUBLISHER_ID_TOKEN` environment variable if
found (this may be used on GitLab CI/CD). Finally it also accepts an ID token
as an extra argument.

When obtaining an ID token, you must make sure its `aud` claim matches
attic-trusted-publisher's configuration.

## Reverse proxy configuration

It is possible, though optional, to host both attic and attic-trusted-publisher
under the same hostname and port using a reverse proxy. attic-trusted-publisher
only defines endpoints under `/_trusted-publisher`:
```nix
{
  services.nginx.enable = true;
  services.nginx.virtualHosts."attic.example.com" = {
    locations."/" = {
      proxyPass = "http://127.0.0.1:3000";
    };
    locations."/_trusted-publisher" = {
      proxyPass = "http://127.0.0.1:3001";
    };
  };

  # Attic wants to know its external URL
  services.attic.settings.api-endpoint = "http://attic.example.com/";
}
```

This allows a single URL to be used:
```
attic login myserver http://attic.example.com $(nix run github:plietar/attic-trusted-publisher login http://attic.example.com)
```
