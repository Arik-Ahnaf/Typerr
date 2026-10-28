#!/usr/bin/env bash
set -euo pipefail
umask 022

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
# Support the project-local tools used for the first build, or normal PATH tools.
if [[ -d "$root/target/windows-tools/llvm/usr/bin" ]]; then
    export PATH="$root/target/windows-tools/llvm/usr/bin:$PATH"
    export LD_LIBRARY_PATH="$root/target/windows-tools/llvm/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi
if [[ -d "$root/target/windows-tools/python/bin" ]]; then
    export PATH="$root/target/windows-tools/python/bin:$PATH"
fi
for tool in cargo cargo-xwin clang-cl lld-link llvm-lib llvm-rc llvm-readobj python3; do
    command -v "$tool" >/dev/null || { echo "Missing $tool; see BUILD.md." >&2; exit 1; }
done
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
export CARGO_TARGET_DIR="$root/target"
export XWIN_CACHE_DIR="$root/target/xwin"
export XWIN_ARCH=x86_64
# Link the C runtime statically; never apply this flag to native Arch builds.
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS='-C target-feature=+crt-static'
unset CARGO_BUILD_TARGET

cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo xwin clippy --locked --release --target x86_64-pc-windows-msvc --all-targets -- -D warnings
# Strip final-binary debug info so the portable release needs no Microsoft PDBs.
cargo xwin rustc --locked --release --bin typerr --target x86_64-pc-windows-msvc \
    -- -C strip=debuginfo -C link-arg=/DEBUG:NONE

mkdir -p target/packaging dist
work=$(mktemp -d "$root/target/packaging/windows.XXXXXX")
cargo metadata --locked --offline --format-version 1 \
    --filter-platform x86_64-pc-windows-msvc > "$work/metadata.json"
version=$(python3 -c 'import pathlib,tomllib; print(tomllib.loads(pathlib.Path("Cargo.toml").read_text())["package"]["version"])')
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || exit 1
name="typerr-${version}-windows-x86_64"
payload="$work/$name"
mkdir -p "$payload/notices/rust"
binary="$root/target/x86_64-pc-windows-msvc/release/typerr.exe"
llvm-readobj --file-headers --coff-imports "$binary" > "$work/pe-inspection.txt"
grep -q 'IMAGE_FILE_MACHINE_AMD64' "$work/pe-inspection.txt"
grep -q 'IMAGE_SUBSYSTEM_WINDOWS_GUI' "$work/pe-inspection.txt"
if grep -Ei 'Name: (VCRUNTIME|MSVCP|MSVCR|ucrtbase|api-ms-win-crt)[^ ]*\.dll' "$work/pe-inspection.txt"; then
    echo 'Unexpected dynamic MSVC runtime dependency.' >&2; exit 1
fi
install -m755 "$binary" "$payload/typerr.exe"
install -m644 settings.json "$payload/settings.json"
install -m644 packaging/windows/README.md "$payload/README.md"
install -m644 assets/fonts/Google_Sans_Code/OFL.txt "$payload/notices/Google-Sans-Code-OFL.txt"
install -m644 assets/fonts/Jomhuria/OFL.txt "$payload/notices/Jomhuria-OFL.txt"
install -m644 assets/words/README.md "$payload/notices/WORDS.md"
install -m644 packaging/LICENSE-STATUS.txt "$payload/notices/LICENSE-STATUS.txt"
python3 packaging/collect-notices.py "$work/metadata.json" "$payload/notices"
rust_docs="$(rustc --print sysroot)/share/doc/rust"
install -m644 "$rust_docs/COPYRIGHT-library.html" "$payload/notices/rust/COPYRIGHT-library.html"
cp -r "$rust_docs/licenses" "$payload/notices/rust/licenses"

python3 - "$payload" "$root/dist" <<'PY'
from pathlib import Path
import hashlib
import sys
import zipfile

payload, dist = map(Path, sys.argv[1:])
archive = payload.parent / (payload.name + ".zip")
with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as bundle:
    for path in sorted(payload.rglob("*")):
        if path.is_file():
            bundle.write(path, path.relative_to(payload.parent))
with zipfile.ZipFile(archive) as bundle:
    assert bundle.testzip() is None
archive.replace(dist / archive.name)
artifacts = sorted(p for p in dist.glob("typerr-*") if p.is_file() and
                   p.name.endswith((".tar.gz", ".pkg.tar.zst", ".zip")))
(dist / "SHA256SUMS").write_text("".join(
    f"{hashlib.file_digest(p.open('rb'), 'sha256').hexdigest()}  {p.name}\n"
    for p in artifacts))
PY
printf 'Windows ZIP: %s/dist/%s.zip\nBuild evidence: %s\n' "$root" "$name" "$work"
