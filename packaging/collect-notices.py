#!/usr/bin/env python3
"""Collect target-filtered locked dependency notices without fetching."""

import json
from pathlib import Path
import shutil
import sys


def main():
    metadata = json.loads(Path(sys.argv[1]).read_text())
    destination = Path(sys.argv[2])
    packaging = Path(__file__).resolve().parent
    supplements = json.loads((packaging / "notices.json").read_text())
    resolved = {node["id"] for node in metadata["resolve"]["nodes"]}
    index = ["# Third-party dependency notices", "",
             "Includes the target-filtered locked dependency graph, including build tools.",
             "License expressions are upstream metadata; original notices follow in crates/.", ""]
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        if package["id"] not in resolved or package["name"] == "typerr":
            continue
        key = f'{package["name"]}@{package["version"]}'
        source = Path(package["manifest_path"]).parent
        files = sorted(p for p in source.rglob("*") if p.is_file() and any(
            word in p.name.lower() for word in ("license", "licence", "copying", "copyright", "notice")))
        if package.get("license_file"):
            files = sorted(set(files + [source / package["license_file"]]))
        target = destination / "crates" / key
        index.extend([f'## {key}', f'License: {package.get("license") or "See license file"}',
                      f'Source: {package.get("repository") or package["source"]}',
                      f'Authors: {", ".join(package["authors"]) or "See original notices"}'])
        if not files:
            supplement = supplements[key]  # Fail rather than silently omit a notice.
            original = packaging / supplement["file"]
            target.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(original, target / original.name)
            index.append(f'Supplemental notice: {supplement["source"]}')
            if supplement.get("additional_file"):
                shutil.copyfile(packaging / supplement["additional_file"], target / "LICENSE-APACHE")
                index.append(f'Selected license: {supplement["selected_license"]}; see LICENSE-APACHE.')
            if supplement.get("attribution_file"):
                shutil.copyfile(packaging / supplement["attribution_file"], target / "ATTRIBUTIONS.txt")
        for original in files:
            output = target / original.relative_to(source)
            output.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(original, output)
        index.append("")
    destination.mkdir(parents=True, exist_ok=True)
    (destination / "THIRD-PARTY.md").write_text("\n".join(index))


if __name__ == "__main__":
    main()
