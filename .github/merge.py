#!/usr/bin/env python3
# Merges the per-platform latest.json files and collects all installers into a
# single release/ dir for upload.
import json
import os
import shutil
import glob

os.makedirs("release", exist_ok=True)

merged = {"platforms": {}}
for lj in sorted(glob.glob("artifacts/*/latest.json")):
    d = json.load(open(lj))
    merged["platforms"].update(d.get("platforms", {}))
    if "version" not in merged:
        merged["version"] = d.get("version")
        merged["notes"] = d.get("notes", "")
        merged["pub_date"] = d.get("pub_date", "")

for f in glob.glob("artifacts/*/*"):
    if f.endswith(".json"):
        continue
    shutil.copy(f, os.path.join("release", os.path.basename(f)))

json.dump(merged, open("release/latest.json", "w"), indent=2)
print("merged platforms:", list(merged["platforms"].keys()))
