#!/usr/bin/env python3
# Renames the built installers to the exact required names and GENERATES the
# latest.json (Tauri's `tauri build` produces .sig files but NOT latest.json —
# that was tauri-action's job). Signatures are read from the .sig files and
# written inline; no .sig files are uploaded.
import datetime
import json
import os
import shutil

version = json.load(open("src-tauri/tauri.conf.json"))["version"]
suffix = os.environ.get("SUFFIX", "")  # "" (stable) or "-prerelease"
tag = os.environ["TAG"]  # e.g. "v0.0.2" or "v0.0.2-prerelease"
bundle = "src-tauri/target/release/bundle"

# Extension -> (exact output name, list of latest.json platform keys).
ext_map = {
    ".AppImage": (f"Roans-Terminal-{version}{suffix}-amd64.AppImage", ["linux-x86_64", "linux-x86_64-appimage"]),
    ".deb": (f"Roans-Terminal-{version}{suffix}-amd64.deb", ["linux-x86_64-deb"]),
    ".rpm": (f"Roans-Terminal-{version}{suffix}-x86-64.rpm", ["linux-x86_64-rpm"]),
    ".dmg": (f"Roans-Terminal-{version}{suffix}-aarch64.dmg", ["darwin-aarch64"]),
    ".msi": (f"Roans-Terminal-{version}{suffix}-x64.msi", ["windows-x86_64", "windows-x86_64-msi"]),
}

os.makedirs("out", exist_ok=True)
platforms = {}

for root, _, files in os.walk(bundle):
    for f in files:
        if f.endswith(".sig"):
            continue
        ext = os.path.splitext(f)[1]
        if ext not in ext_map:
            continue
        new_name, keys = ext_map[ext]
        src = os.path.join(root, f)
        shutil.copy(src, os.path.join("out", new_name))

        sig_path = src + ".sig"
        signature = open(sig_path, "r").read().strip() if os.path.exists(sig_path) else ""
        url = f"https://github.com/roanh47/Roans-Terminal/releases/download/{tag}/{new_name}"
        for key in keys:
            platforms[key] = {"signature": signature, "url": url}

latest = {
    "version": version,
    "notes": "",
    "pub_date": datetime.datetime.now(datetime.timezone.utc).isoformat().replace("+00:00", "Z"),
    "platforms": platforms,
}

json.dump(latest, open("out/latest.json", "w"), indent=2)
print("generated latest.json platforms:", list(platforms.keys()))
