import sys
from datetime import datetime, UTC
from urllib.parse import urljoin, urlunsplit

import bottle
from jwcrypto.jwk import JWK, JWKSet
from jwcrypto.jwt import JWT

KEY = JWK.generate(kty="RSA")
KEY['kid'] = KEY.thumbprint()
KEY['alg'] = 'RS256'

KEYSET = JWKSet(keys=KEY)


@bottle.route("/.well-known/openid-configuration")
def oidc_configuration():
    return {
        "jwks_uri": urljoin(bottle.request.url, "/jwks.json"),
    }


@bottle.route("/jwks.json")
def jwks():
    return KEYSET.export(as_dict=True, private_keys=False)


@bottle.post("/token")
def token():
    claims = bottle.request.json
    iss = urlunsplit(bottle.request.urlparts._replace(path=""))
    claims.setdefault("iss", iss)
    claims.setdefault("exp", int(datetime.now(UTC).timestamp()) + 3600)
    jwt = JWT(header={"alg": "RS256", "kid": KEY["kid"]}, claims=claims)
    jwt.make_signed_token(KEY)

    bottle.response.content_type = "application/jwt"
    return jwt.serialize()


port = int(sys.argv[1])
bottle.run(host="0.0.0.0", port=port)
