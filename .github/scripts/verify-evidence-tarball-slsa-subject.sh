#!/usr/bin/env bash
# Compare tarball bytes on disk to SLSA generic provenance subjects when such
# provenance is present in the evidence bundle. GitHub build provenance is
# persisted as an attestation, not as a normal workflow artifact, so this check
# is optional until a downloadable DSSE/JSONL provenance artifact is available.
set -euo pipefail
EVIDENCE="${1:?evidence directory}"
TAR="$(find "$EVIDENCE/chassis-source-archive" -name '*.tar.gz' -print -quit || true)"
if [[ ! -f "${TAR:-}" ]]; then
	echo "CH-EVIDENCE-DIGEST-MISMATCH: expected tarball under evidence/chassis-source-archive" >&2
	exit 1
fi
shopt -s nullglob
PROVS=("$EVIDENCE"/slsa-provenance/*.intoto.jsonl)
shopt -u nullglob
if [[ ${#PROVS[@]} -eq 0 ]]; then
	echo "evidence-digest: SKIP no downloadable SLSA provenance under evidence/slsa-provenance"
	exit 0
fi
PROV="${PROVS[0]}"
python3 - "$TAR" "$PROV" <<'PY'
import base64
import hashlib
import json
import sys
from pathlib import Path

tar_path = Path(sys.argv[1])
prov_path = Path(sys.argv[2])
got = hashlib.sha256(tar_path.read_bytes()).hexdigest().lower()
line = prov_path.read_text(encoding="utf-8").splitlines()[0]
envelope = json.loads(line)
payload = envelope.get("payload")
if not isinstance(payload, str):
    print("CH-EVIDENCE-DIGEST-MISMATCH: DSSE payload is not a base64 string", file=sys.stderr)
    raise SystemExit(1)
stmt = json.loads(base64.b64decode(payload))
for sub in stmt.get("subject", []):
    dig = (sub.get("digest") or {}).get("sha256")
    if isinstance(dig, str) and dig.lower() == got:
        print(f"evidence-digest: OK tarball sha256 matches SLSA subject ({got})")
        raise SystemExit(0)
print(
    f"CH-EVIDENCE-DIGEST-MISMATCH: tarball sha256={got} not found in SLSA subjects",
    file=sys.stderr,
)
raise SystemExit(1)
PY
