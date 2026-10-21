#!/bin/sh
# Packages a release build for Linux or macOS into dist/: the raw binary used by
# install.sh and self-update, and a tar.gz with the binary, documentation and
# installer scripts (plus RustCalc.app on macOS).
#
#   sh scripts/package.sh                       # binary from target/release
#   sh scripts/package.sh x86_64-apple-darwin   # binary from target/<triple>/release
set -eu

root="$(cd "$(dirname "$0")/.." && pwd)"
version="$(sed -n 's/^version = "\([0-9.]*\)"/\1/p' "$root/Cargo.toml" | head -n 1)"
triple="${1:-}"
if [ -n "$triple" ]; then
    binary="$root/target/$triple/release/rustcalc"
else
    binary="$root/target/release/rustcalc"
    case "$(uname -s)-$(uname -m)" in
        Linux-x86_64) triple=x86_64-unknown-linux-gnu ;;
        Darwin-arm64) triple=aarch64-apple-darwin ;;
        Darwin-x86_64) triple=x86_64-apple-darwin ;;
        *) echo "Unsupported build host" >&2; exit 1 ;;
    esac
fi
case "$triple" in
    x86_64-unknown-linux-gnu) platform=linux-x86_64 ;;
    aarch64-apple-darwin) platform=macos-aarch64 ;;
    x86_64-apple-darwin) platform=macos-x86_64 ;;
    *) echo "No release platform for $triple" >&2; exit 1 ;;
esac
[ -x "$binary" ] || { echo "Build first: cargo build --release --locked${1:+ --target $1}" >&2; exit 1; }

dist="$root/dist"
mkdir -p "$dist"
cp "$binary" "$dist/rustcalc-$platform"
chmod 755 "$dist/rustcalc-$platform"
cp "$root/install.sh" "$root/uninstall.sh" "$dist/"

stage="$(mktemp -d "${TMPDIR:-/tmp}/rustcalc-package-XXXXXX")"
trap 'rm -rf "$stage"' EXIT
folder="rustcalc-v$version-$platform"
mkdir -p "$stage/$folder"
cp "$binary" "$stage/$folder/rustcalc"
cp "$root/README.md" "$root/LICENSE" "$root/CHANGELOG.md" "$root/install.sh" "$root/uninstall.sh" "$stage/$folder/"
if [ "${platform%%-*}" = macos ]; then
    # A self-contained app for drag-and-drop installs; self-update replaces the
    # binary inside it.
    app="$stage/$folder/RustCalc.app/Contents"
    mkdir -p "$app/MacOS"
    cp "$binary" "$app/MacOS/rustcalc"
    cat > "$app/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>RustCalc</string>
<key>CFBundleIdentifier</key><string>io.github.ersinkoc.rustcalc</string>
<key>CFBundleExecutable</key><string>rustcalc</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
EOF
fi
tar -C "$stage" -czf "$dist/$folder.tar.gz" "$folder"
echo "Packaged RustCalc v$version for $platform in $dist"
