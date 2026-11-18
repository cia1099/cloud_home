#!/usr/bin/env python3
"""Generate manifest.json for a release from the built tarballs.

usage: make_manifest.py <version> <repo> <dist_dir> [min_from_version]
Tarballs must be named cloud_home-<version>-<target>.tar.gz
"""
import hashlib, json, pathlib, re, sys, datetime

version, repo, dist = sys.argv[1], sys.argv[2], pathlib.Path(sys.argv[3])
min_from = sys.argv[4] if len(sys.argv) > 4 else "0.0.0"
prefix = f"cloud_home-{version}-"

targets = {}
for f in sorted(dist.glob(f"{prefix}*.tar.gz")):
    target = f.name[len(prefix):-len(".tar.gz")]
    targets[target] = {
        "url": f"https://github.com/{repo}/releases/download/v{version}/{f.name}",
        "sha256": hashlib.sha256(f.read_bytes()).hexdigest(),
        "size": f.stat().st_size,
    }
if not targets:
    sys.exit("no tarballs found in " + str(dist))

manifest = {
    "version": version,
    "released_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "notes_url": f"https://github.com/{repo}/releases/tag/v{version}",
    "min_from_version": min_from,
    "targets": targets,
}
(dist / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(json.dumps(manifest, indent=2))
