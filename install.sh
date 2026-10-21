#!/bin/sh
# Installs a SHA-256-verified RustCalc release for the current user on Linux
# (x86_64) or macOS (Apple silicon or Intel). No administrator access needed.
#
#   sh install.sh                        # latest stable release
#   sh install.sh --version 0.5.0
#   sh install.sh --from-directory ./downloads --version 0.5.0
#
# Options:
#   --version X.Y.Z        Install a specific stable release
#   --install-dir DIR      Program folder (default ~/.local/opt/rustcalc)
#   --bin-dir DIR          Folder for the rustcalc command link (default ~/.local/bin)
#   --from-directory DIR   Use already downloaded release files (needs --version)
#   --no-link              Do not create the rustcalc command link
#   --no-desktop           Do not add a menu entry (Linux) or app (macOS)
#
# History and preferences live elsewhere (~/.local/share/rustcalc on Linux,
# ~/Library/Application Support/RustCalc on macOS) and are never touched.
set -eu

repo="ersinkoc/rustcalc"
version=""
install_dir="$HOME/.local/opt/rustcalc"
bin_dir="$HOME/.local/bin"
from_dir=""
link=1
desktop=1

fail() {
    printf 'RustCalc installer: %s\n' "$1" >&2
    exit 1
}
value() {
    [ "$#" -ge 2 ] && [ -n "$2" ] || fail "$1 needs a value"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --version) value "$@"; version="$2"; shift 2 ;;
        --install-dir) value "$@"; install_dir="$2"; shift 2 ;;
        --bin-dir) value "$@"; bin_dir="$2"; shift 2 ;;
        --from-directory) value "$@"; from_dir="$2"; shift 2 ;;
        --no-link) link=0; shift ;;
        --no-desktop) desktop=0; shift ;;
        -h | --help) sed -n '2,19p' "$0"; exit 0 ;;
        *) fail "unknown option: $1" ;;
    esac
done

case "$(uname -s)" in
    Linux) os=linux ;;
    Darwin) os=macos ;;
    *) fail "this installer supports Linux and macOS; use install.ps1 on Windows" ;;
esac
arch="$(uname -m)"
# A Terminal running under Rosetta reports x86_64 on Apple silicon.
if [ "$os" = macos ] && [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" = 1 ]; then
    arch=arm64
fi
case "$os-$arch" in
    linux-x86_64 | linux-amd64) asset="rustcalc-linux-x86_64" ;;
    macos-arm64 | macos-aarch64) asset="rustcalc-macos-aarch64" ;;
    macos-x86_64) asset="rustcalc-macos-x86_64" ;;
    *) fail "no official binary for $os $arch; build from source with cargo" ;;
esac

case "$install_dir" in
    /*) ;;
    *) install_dir="$(pwd)/$install_dir" ;;
esac
case "${install_dir%/}" in
    "" | "$HOME" | "$HOME/.local" | "$HOME/.local/share") fail "choose an application directory, not $install_dir" ;;
esac
marker="$install_dir/install.json"
if [ -f "$marker" ]; then
    grep -q '"product": "RustCalc"' "$marker" || fail "the directory has a conflicting installation marker"
elif [ -d "$install_dir" ] && [ -n "$(ls -A "$install_dir")" ]; then
    fail "the installation directory is not empty and is not a RustCalc installation"
fi

download() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --proto '=https' --tlsv1.2 -H 'User-Agent: RustCalc-Installer' -o "$2" "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget -q --https-only -U RustCalc-Installer -O "$2" "$1"
    else
        fail "curl or wget is required"
    fi
}
sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d ' ' -f 1
    else
        shasum -a 256 "$1" | cut -d ' ' -f 1
    fi
}
is_stable() {
    printf '%s\n' "$1" | grep -Eq '^v?(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$'
}
json_string() {
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

if [ -n "$version" ]; then
    is_stable "$version" || fail "version must be a stable version such as 0.5.0"
    version="${version#v}"
elif [ -n "$from_dir" ]; then
    fail "--from-directory requires --version"
fi

stage="$(mktemp -d "${TMPDIR:-/tmp}/rustcalc-install-XXXXXX")"
trap 'rm -rf "$stage"' EXIT INT TERM

if [ -z "$version" ]; then
    download "https://api.github.com/repos/$repo/releases/latest" "$stage/latest.json" ||
        fail "could not find a stable release; see https://github.com/$repo/releases or use --version"
    tag="$(sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' "$stage/latest.json" | head -n 1)"
    is_stable "$tag" || fail "the latest release is not a valid stable release"
    version="${tag#v}"
fi

for name in SHA256SUMS "$asset" uninstall.sh; do
    if [ -n "$from_dir" ]; then
        cp "$from_dir/$name" "$stage/$name" 2>/dev/null || fail "missing $name in $from_dir"
    else
        download "https://github.com/$repo/releases/download/v$version/$name" "$stage/$name" ||
            fail "could not download $name for version $version"
    fi
done

for name in "$asset" uninstall.sh; do
    expected="$(awk -v n="$name" '$2 == n || $2 == "*" n { print tolower($1) }' "$stage/SHA256SUMS")"
    [ "$(printf '%s' "$expected" | grep -c .)" -eq 1 ] || fail "expected exactly one checksum for $name"
    [ "$(sha256 "$stage/$name")" = "$expected" ] ||
        fail "SHA-256 verification failed for $name; the installed application was not changed"
done
[ "$(wc -c < "$stage/$asset")" -le 67108864 ] || fail "the release executable exceeds the size limit"
chmod 755 "$stage/$asset"
[ "$("$stage/$asset" --version 2>/dev/null || true)" = "rustcalc $version" ] ||
    fail "the executable version does not match the requested release"

mkdir -p "$install_dir/bin"
target="$install_dir/bin/rustcalc"
pending="$install_dir/bin/.rustcalc.pending.$$"
cp "$stage/$asset" "$pending"
chmod 755 "$pending"
# rename() swaps the file atomically, even while RustCalc is running.
mv -f "$pending" "$target"
[ "$("$target" --version 2>/dev/null || true)" = "rustcalc $version" ] ||
    fail "installed executable verification failed"
cp "$stage/uninstall.sh" "$install_dir/uninstall.sh"
chmod 755 "$install_dir/uninstall.sh"

# Every file created outside the program folder is recorded for uninstall.
created="$install_dir/created"
: > "$created"
if [ "$link" = 1 ]; then
    mkdir -p "$bin_dir"
    if [ -e "$bin_dir/rustcalc" ] && [ ! -L "$bin_dir/rustcalc" ]; then
        printf 'Kept the existing %s; it is not a RustCalc link.\n' "$bin_dir/rustcalc"
    else
        ln -sfn "$target" "$bin_dir/rustcalc"
        printf '%s\n' "$bin_dir/rustcalc" >> "$created"
    fi
fi
if [ "$desktop" = 1 ] && [ "$os" = linux ]; then
    data="${XDG_DATA_HOME:-$HOME/.local/share}"
    icon="$data/icons/hicolor/64x64/apps/rustcalc.png"
    entry="$data/applications/rustcalc.desktop"
    mkdir -p "$(dirname "$icon")" "$(dirname "$entry")"
    printf '%s' 'iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAYAAACqaXHeAAAAh0lEQVR42u3bsQ3AIAwEQC+BIjpWyO6R2Ig1YAHKSEHxFV8hubiGd+Eotc7MCQAAAGwf+nh+FQAAAAAAAAAAgNcArrsdFQAAAAAAAAAAAAAAAFiGAAAAAAAAgEN7wFdzAQAAAAAAAACKEAAAAAAA8A0CAAAAAAAAABQhAAAAAADgcBIAgMQAC+uuAvwYofS7AAAAAElFTkSuQmCC' |
        base64 -d > "$icon"
    {
        printf '[Desktop Entry]\nType=Application\nName=RustCalc\n'
        printf 'Comment=Native desktop calculator\nExec="%s"\nIcon=%s\n' "$target" "$icon"
        printf 'Terminal=false\nCategories=Utility;Calculator;\n'
    } > "$entry"
    printf '%s\n%s\n' "$entry" "$icon" >> "$created"
elif [ "$desktop" = 1 ] && [ "$os" = macos ]; then
    app="$HOME/Applications/RustCalc.app"
    mkdir -p "$app/Contents/MacOS"
    quoted="$(printf '%s' "$target" | sed "s/'/'\\\\''/g")"
    printf "#!/bin/sh\nexec '%s' \"\$@\"\n" "$quoted" > "$app/Contents/MacOS/RustCalc"
    chmod 755 "$app/Contents/MacOS/RustCalc"
    cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>RustCalc</string>
<key>CFBundleIdentifier</key><string>io.github.ersinkoc.rustcalc</string>
<key>CFBundleExecutable</key><string>RustCalc</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
EOF
    printf '%s\n' "$app" >> "$created"
fi

printf '{\n  "product": "RustCalc",\n  "installDir": "%s",\n  "version": "%s"\n}\n' \
    "$(json_string "$install_dir")" "$version" > "$marker"

printf 'RustCalc %s installed to %s\n' "$version" "$target"
case ":$PATH:" in
    *":$bin_dir:"*) [ "$link" = 1 ] && printf 'Run: rustcalc\n' ;;
    *) [ "$link" = 1 ] && printf 'Add %s to your PATH to run rustcalc from any directory.\n' "$bin_dir" ;;
esac
printf 'Update: rustcalc update | Uninstall: sh "%s/uninstall.sh"\n' "$install_dir"
