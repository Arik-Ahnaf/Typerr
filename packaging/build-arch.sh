#!/usr/bin/env bash
set -euo pipefail
umask 022

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
source /etc/os-release
[[ $ID == arch && $(uname -m) == x86_64 ]] || {
    echo 'Build on native Arch Linux x86_64.' >&2; exit 1;
}
for tool in cargo rustc python3 makepkg tar gzip sha256sum file ldd desktop-file-validate; do
    command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 1; }
done
[[ $(rustc -vV) == *'host: x86_64-unknown-linux-gnu'* ]] || exit 1
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
# Keep output paths independent of user Cargo environment overrides.
export CARGO_TARGET_DIR="$root/target"
unset CARGO_BUILD_TARGET
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
cargo build --locked --release

mkdir -p "$root/target/packaging" "$root/dist"
work=$(mktemp -d "$root/target/packaging/arch.XXXXXX")
cargo metadata --locked --offline --format-version 1 \
    --filter-platform x86_64-unknown-linux-gnu > "$work/metadata.json"
version=$(python3 -c 'import pathlib,tomllib; print(tomllib.loads(pathlib.Path("Cargo.toml").read_text())["package"]["version"])')
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Unsupported package version' >&2; exit 1; }
name="typerr-${version}-arch-linux-x86_64"
payload="$work/$name"
mkdir -p "$payload/notices"
binary="$root/target/release/typerr"
file "$binary" | tee "$work/binary.txt"
file "$binary" | grep -q 'ELF 64-bit.*x86-64'
ldd "$binary" | tee "$work/linkage.txt"
if grep -q 'not found' "$work/linkage.txt"; then exit 1; fi
install -m755 "$binary" "$payload/typerr"
install -m644 settings.json "$payload/settings.json"
install -m644 packaging/README.md "$payload/README.md"
install -m644 packaging/arch/typerr.desktop "$payload/typerr.desktop"
install -m644 assets/icons/typerr-logo.svg "$payload/typerr-logo.svg"
desktop-file-validate "$payload/typerr.desktop"
install -m644 assets/fonts/Google_Sans_Code/OFL.txt "$payload/notices/Google-Sans-Code-OFL.txt"
install -m644 assets/fonts/Jomhuria/OFL.txt "$payload/notices/Jomhuria-OFL.txt"
install -m644 assets/words/README.md "$payload/notices/WORDS.md"
install -m644 packaging/LICENSE-STATUS.txt "$payload/notices/LICENSE-STATUS.txt"
python3 packaging/collect-notices.py "$work/metadata.json" "$payload/notices"
rust_docs="$(rustc --print sysroot)/share/doc/rust"
mkdir -p "$payload/notices/rust"
install -m644 "$rust_docs/COPYRIGHT-library.html" "$payload/notices/rust/COPYRIGHT-library.html"
cp -r "$rust_docs/licenses" "$payload/notices/rust/licenses"

archive="$name.tar.gz"
tar --sort=name --owner=0 --group=0 --numeric-owner -C "$work" -czf "$work/$archive" "$name"
checksum=$(sha256sum "$work/$archive" | cut -d ' ' -f1)
sed -e "s/@VERSION@/$version/g" -e "s/@SHA256@/$checksum/g" \
    packaging/arch/PKGBUILD > "$work/PKGBUILD"
(
    cd "$work"
    # No install, dependency installation, signing, or debug companion package.
    PKGDEST="$work" SRCDEST="$work" BUILDDIR="$work" \
        makepkg --force --nosign
)
package="typerr-${version}-1-x86_64.pkg.tar.zst"
[[ -s "$work/$package" ]]
install -m644 "$work/$archive" "$work/$package" "$root/dist/"
(
    cd "$root/dist"
    shopt -s nullglob
    sha256sum typerr-*.tar.gz typerr-*.pkg.tar.zst typerr-*.zip > SHA256SUMS
    sha256sum --check SHA256SUMS
)
printf 'Packages: %s/dist\nBuild evidence: %s\n' "$root" "$work"
