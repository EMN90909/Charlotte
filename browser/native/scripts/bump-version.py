#!/usr/bin/env python3
"""Bump Charlotte Native version; do not create git commits or tags."""
import pathlib, re, sys
root = pathlib.Path(__file__).resolve().parents[2]
part = sys.argv[1] if len(sys.argv) == 2 else ""
if part not in {"major", "minor", "patch"}:
    sys.exit("usage: bump-version.py major|minor|patch")
cargo = root / "native" / "Cargo.toml"
data = cargo.read_text()
match = re.search(r'(?m)^version = "(\d+)\.(\d+)\.(\d+)"$', data)
if match is None:
    sys.exit("missing native Cargo version")
a, b, c = map(int, match.groups())
if part == "major":
    a, b, c = a + 1, 0, 0
elif part == "minor":
    b, c = b + 1, 0
else:
    c += 1
new = f"{a}.{b}.{c}"
old = match.group(0)
cargo.write_text(data.replace(old, f'version = "{new}"', 1))
installer = root / "native/packaging/windows/charlotte.iss"
installer.write_text(re.sub(r'(?m)^#define AppVersion ".*"$',
                            f'#define AppVersion "{new}"', installer.read_text()))
build = root / "native/scripts/build.sh"
build.write_text(re.sub(r'--version \d+\.\d+\.\d+', f'--version {new}', build.read_text()))
print(new)
