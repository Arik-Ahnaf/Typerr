#!/usr/bin/env bash
set -euo pipefail
umask 022

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
for tool in python3 7z sha256sum; do
    command -v "$tool" >/dev/null || { echo "Missing $tool; see BUILD.md." >&2; exit 1; }
done

version=$(python3 -c 'import pathlib,tomllib; print(tomllib.loads(pathlib.Path("Cargo.toml").read_text())["package"]["version"])')
zip="$root/dist/typerr-${version}-windows-x86_64.zip"
[[ -s $zip ]] || { echo "Build $zip first." >&2; exit 1; }
work=$(mktemp -d "$root/target/packaging/windows-installer.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
7z x -y -o"$work/payload" "$zip" >/dev/null
payload="$work/payload/typerr-${version}-windows-x86_64"
[[ -x $payload/typerr.exe && -s $payload/settings.json ]] || { echo 'Invalid Windows payload.' >&2; exit 1; }
default_settings="$root/target/packaging/windows-installer-settings.json"
install -m644 "$payload/settings.json" "$default_settings"
rm "$payload/settings.json"

output="$root/dist/typerr-${version}-setup.exe"
nsis_output=$output
nsis_icon="$root/assets/icons/typerr-logo.ico"
nsis_payload=$payload
nsis_settings=$default_settings
script_arg="$work/installer.nsi"
if [[ -n ${MAKENSIS:-} ]]; then
    compiler=("$MAKENSIS")
elif command -v makensis >/dev/null; then
    compiler=(makensis)
elif [[ -x $root/target/windows-tools/nsis/makensis.exe ]] && command -v wine >/dev/null; then
    compiler=(wine "$root/target/windows-tools/nsis/makensis.exe")
    nsis_output=$(winepath -w "$output")
    nsis_icon=$(winepath -w "$nsis_icon")
    nsis_payload=$(winepath -w "$payload")
    nsis_settings=$(winepath -w "$default_settings")
    nsis_output=${nsis_output//\\/\\\\}
    nsis_icon=${nsis_icon//\\/\\\\}
    nsis_payload=${nsis_payload//\\/\\\\}
    nsis_settings=${nsis_settings//\\/\\\\}
    script_arg=$(winepath -w "$script_arg")
else
    echo 'Missing makensis; see BUILD.md.' >&2; exit 1
fi
sed -e "s|@VERSION@|$version|g" \
    -e "s|@OUTPUT@|$nsis_output|g" \
    -e "s|@ICON@|$nsis_icon|g" \
    -e "s|@PAYLOAD@|$nsis_payload|g" \
    -e "s|@SETTINGS@|$nsis_settings|g" \
    packaging/windows/installer.nsi > "$work/installer.nsi"
install -m644 "$work/installer.nsi" "$root/target/packaging/windows-installer-last.nsi"
"${compiler[@]}" "$script_arg"
[[ -s $output ]]
(
    cd "$root/dist"
    shopt -s nullglob
    sha256sum typerr-*.tar.gz typerr-*.pkg.tar.zst typerr-*.zip typerr-*-setup.exe > SHA256SUMS
    sha256sum --check SHA256SUMS
)
printf 'Windows installer: %s\n' "$output"
