# Chassis release / verification keys

Any `*.pub` committed under `.chassis/keys/` is a **public verifier** intended for
consumers to pin (for example `.chassis/keys/release.pub` paired with a
gitignored `.chassis/keys/release.priv`).

- **Never commit** private key material (`*.priv`, raw seeds, or PKCS#8 blobs).
  `foundation.yml` fails closed if `git ls-files` reports any `*.priv` path.
- Rotate keys per your org policy; update the committed `.pub` when you rotate.
