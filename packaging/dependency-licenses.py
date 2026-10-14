#!/usr/bin/env python3
"""List the licenses of the crates compiled into mdview.

Walks `cargo metadata` from the root package through normal and build
dependencies (dev-dependencies are not shipped) for Linux, and prints one line
per crate. With --spec, prints a single SPDX expression for the License tag of
the RPM spec instead.
"""

import json
import re
import subprocess
import sys

# Crates that do not declare a license in their metadata, with the license of
# their repository's LICENSE file.
LICENSE_OVERRIDES = {
    "apply": "Unlicense",
    "cosmic-config": "MPL-2.0",
    "cosmic-config-derive": "MPL-2.0",
    "cosmic-settings-daemon": "MPL-2.0",
    "cosmic-theme": "MPL-2.0",
    "dnd": "MIT",
    "iced_accessibility": "MIT",
    "libcosmic": "MPL-2.0",
    "mime": "MIT",
}


def shipped_packages(metadata):
    """Packages reachable from the root without going through dev-dependencies."""
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    root = metadata["resolve"]["root"]
    seen, stack = set(), [root]
    while stack:
        node = nodes[stack.pop()]
        for dep in node["deps"]:
            kinds = {kind["kind"] for kind in dep["dep_kinds"]}
            if kinds - {"dev"} and dep["pkg"] not in seen:
                seen.add(dep["pkg"])
                stack.append(dep["pkg"])
    return [pkg for pkg in metadata["packages"] if pkg["id"] in seen]


def normalize(expression):
    """Rewrites the old `A/B` notation as `A OR B` and orders the alternatives of
    plain OR-expressions, so equivalent expressions compare equal."""
    expression = re.sub(r"\s*/\s*", " OR ", expression.strip())
    if " AND " in expression or "(" in expression or " WITH " in expression:
        return expression
    return " OR ".join(sorted(set(expression.split(" OR "))))


def spec_license(own, licenses):
    """The License tag: the package's own license and every distinct dependency
    license, each compound expression in parentheses."""
    expressions = sorted({normalize(license) for license in licenses} - {own})
    parts = [own] + [f"({e})" if " " in e else e for e in expressions]
    return " AND ".join(parts)


def main():
    metadata = json.loads(
        subprocess.check_output(
            [
                "cargo", "metadata", "--format-version", "1", "--locked",
                "--filter-platform", "x86_64-unknown-linux-gnu",
            ]
        )
    )
    packages = sorted(shipped_packages(metadata), key=lambda p: (p["name"], p["version"]))
    licenses = [
        (p["name"], p["version"], p.get("license") or LICENSE_OVERRIDES.get(p["name"], "UNKNOWN"))
        for p in packages
    ]
    unknown = [name for name, _, license in licenses if license == "UNKNOWN"]
    if unknown:
        sys.exit(f"crates without a known license: {', '.join(unknown)}")

    if "--spec" in sys.argv:
        root = next(p for p in metadata["packages"] if p["id"] == metadata["resolve"]["root"])
        print(spec_license(root["license"], [license for _, _, license in licenses]))
    else:
        for name, version, license in licenses:
            print(f"{name} {version}: {license}")


if __name__ == "__main__":
    main()
