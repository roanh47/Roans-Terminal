#!/usr/bin/env python3
# Renames the built installers to the exact required names and rewrites
# latest.json URLs to match. Runs per-platform inside each matrix job.
import json
import os
import shutil

version = json.load(open("src-tauri/tauri.conf.json"))["version"]
suffix = os.environ.get("SUFFIX", "")  # "" (stable) or "-prerelease"
bundle = "src-tauri/target/release/bundle"

# Extension -> exact output name (arch follows the package's native convention).
ext_map = {
    ".AppImage": f"Roans-Terminal-{version}{suffix}-amd64.AppImage",
    ".deb": f"Roans-Terminal-{version}{suffix}-amd64.deb",
    ".rpm": f"Roans-Terminal-{version}{suffix}-x86-64.rpm",
    ".dmg": f"Roans-Terminal-{version}{suffix}-aarch64.dmg",
    ".msi": f"Roans-Terminal-{version}{suffix}-x64.msi",
}

os.makedirs("out", exist_ok=True)
old_to_new = {}

for root, _, files in os.walk(bundle):
    for f in files:
        ext = os.path.splitext(f)[1]
        if ext in ext_map:
            new = ext_map[ext]
            shutil.copy(os.path.join(root, f), os.path.join("out", new))
            old_to_new[f] = new

# Rewrite latest.json URLs to point at the renamed files. Signatures are over
# the file bytes, not the name, so they stay valid.
lj_path = os.path.join(bundle, "latest.json")
if os.path.exists(lj_path):
    lj = json.load(open(lj_path))
    for entry in lj.get("platforms", {}).values():
        url = entry.get("url", "")
        for old, new in old_to_new.items():
            url = url.replace(old, new)
        entry["url"] = url
    json.dump(lj, open(os.path.join("out", "latest.json"), "w"), indent=2)

print("renamed:", old_to_new)
