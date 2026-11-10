#!/bin/sh
# Installs a SHA-256-verified RustCalc release for the current user on Linux
# (x86_64 or ARM64) or macOS (Apple silicon or Intel). No administrator access
# needed.
#
#   sh install.sh                        # latest stable release
#   sh install.sh --version 0.8.0
#   sh install.sh --from-directory ./downloads --version 0.8.0
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
if [ "$os" = linux ] && [ "$arch" = aarch64 ] && [ "$(getconf LONG_BIT 2>/dev/null || echo 64)" != 64 ]; then
    fail "this system runs 32-bit programs on a 64-bit ARM kernel; the ARM64 build needs a 64-bit system"
fi
case "$os-$arch" in
    linux-x86_64 | linux-amd64) asset="rustcalc-linux-x86_64" ;;
    linux-aarch64 | linux-arm64) asset="rustcalc-linux-aarch64" ;;
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
write_marker() {
    printf '{\n  "product": "RustCalc",\n  "installDir": "%s",\n  "version": "%s"\n}\n' \
        "$(json_string "$install_dir")" "$1" > "$marker"
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

# Published Linux ARM64 builds start with 0.7.0.
if [ -z "$from_dir" ] && [ "$asset" = rustcalc-linux-aarch64 ] && [ "${version%%.*}" = 0 ]; then
    minor="${version#*.}"
    [ "${minor%%.*}" -ge 7 ] || fail "RustCalc $version has no Linux ARM64 build; install 0.7.0 or later"
fi

for name in SHA256SUMS "$asset" uninstall.sh; do
    if [ -n "$from_dir" ]; then
        cp "$from_dir/$name" "$stage/$name" 2>/dev/null || fail "missing $name in $from_dir"
        # Browser downloads are quarantined on macOS; the checksum below is
        # what establishes trust, so the copy is released from quarantine.
        if [ "$os" = macos ]; then
            xattr -d com.apple.quarantine "$stage/$name" 2>/dev/null || true
        fi
    else
        download "https://github.com/$repo/releases/download/v$version/$name" "$stage/$name" ||
            fail "could not download $name for version $version"
    fi
done

for name in "$asset" uninstall.sh; do
    # Manifests may end lines with CRLF; the carriage return is not part of a name.
    expected="$(awk -v n="$name" '{ sub(/\r$/, "") } $2 == n || $2 == "*" n { print tolower($1) }' "$stage/SHA256SUMS")"
    [ "$(printf '%s' "$expected" | grep -c .)" -eq 1 ] || fail "expected exactly one checksum for $name"
    [ "$(sha256 "$stage/$name")" = "$expected" ] ||
        fail "SHA-256 verification failed for $name; the installed application was not changed"
done
[ "$(wc -c < "$stage/$asset")" -le 67108864 ] || fail "the release executable exceeds the size limit"

# A fresh folder is marked as RustCalc's first, so a failed install can be
# retried or uninstalled instead of leaving an unowned folder behind.
mkdir -p "$install_dir/bin"
# Record the canonical path (no // or symlinked parents, as macOS's /var is),
# so links, the menu entry and uninstall all name the program the same way.
install_dir="$(cd "$install_dir" && pwd -P)"
marker="$install_dir/install.json"
[ -f "$marker" ] || write_marker "$version"
target="$install_dir/bin/rustcalc"
pending="$install_dir/bin/.rustcalc.pending.$$"
trap 'rm -rf "$stage"; rm -f "$pending"' EXIT INT TERM
cp "$stage/$asset" "$pending"
chmod 755 "$pending"
# The candidate runs from the program folder, which works where /tmp is noexec.
[ "$("$pending" --version 2>/dev/null || true)" = "rustcalc $version" ] ||
    fail "the executable version does not match the requested release"
# rename() swaps the file atomically, even while RustCalc is running.
mv -f "$pending" "$target"
[ "$("$target" --version 2>/dev/null || true)" = "rustcalc $version" ] ||
    fail "installed executable verification failed"
cp "$stage/uninstall.sh" "$install_dir/uninstall.sh"
chmod 755 "$install_dir/uninstall.sh"

# Every file created outside the program folder is recorded for uninstall.
# Earlier records are kept, so a reinstall with --no-link still removes the
# link a previous install made; uninstall checks each item's owner anyway.
created="$install_dir/created"
touch "$created"
if [ "$link" = 1 ]; then
    mkdir -p "$bin_dir"
    command_link="$bin_dir/rustcalc"
    # Only a missing entry, a dangling link or our own link is replaced.
    if [ ! -e "$command_link" ] && [ ! -L "$command_link" ] ||
        { [ -L "$command_link" ] && { [ ! -e "$command_link" ] || [ "$(readlink "$command_link")" = "$target" ]; }; }; then
        ln -sfn "$target" "$command_link"
        printf '%s\n' "$command_link" >> "$created"
    else
        printf 'Kept the existing %s; it is not a RustCalc link.\n' "$command_link"
    fi
fi
if [ "$desktop" = 1 ] && [ "$os" = linux ]; then
    data="${XDG_DATA_HOME:-$HOME/.local/share}"
    icon="$data/icons/hicolor/64x64/apps/rustcalc.png"
    entry="$data/applications/rustcalc.desktop"
    mkdir -p "$(dirname "$icon")" "$(dirname "$entry")"
    printf '%s' 'iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAYAAACqaXHeAAACEElEQVR42u1bsU7DMBDtJ2RlZKUjAwsSHZiQYEIMDNAJdayyISFUMTB0RSyIhYGZuVMHxM4HIGVlyyeEPJSg46oWpzmj+HyWnlQ58cX3at+dz3avZ8XK2uXx83XQBfynwkmJcYl5iaJjmFd9S3wpPimRd1BxjrzqayI5zLMAFOfIWk+PUsAwQMU5hjErvx4J1bAvlGHQxOBlCgnInAxjZUELpZi4/Pu5YgLylaOgCiREP3o5e2gFDySMVxEgEuHdvD0XuxfHRdLfFAFkQaZUxLiKABHlN3b6YsrXgEwpEry5vruPmRflKQn4hheXKEHA+f3Vrw4fXo9a2wDIoDLxjc4SsH168NNR/JYyXB7k+ifgZJqKEQBZRkDoBGA+o84F3P8HTwDcV1OLT11e8ARQBVyxrH2QBHD36ALq7oInYPr+0ihAwrtoo8oIIoJzDX54tGdu0AgwAsImAPOaGrmto72Ftqij79BgSGUcwNuqjgOiJyD6KWBeIHYCENntp2ffz4HR0+1CW9TVz/EujQZVLob+MoKqFkPRewEMb77a4235apFOExVGEEN6WcqLp8x46tu8gAYCkOOr1/s02UGTJvVzvgWmLhJ0sQG2GLLFUKorKVpHeQA2PHlb1NF3gk+KRrk5Gv32eNAHJKI/ImOHpOyYnB2UtKOydljajsvbhQm7MmOXpuzanF2ctKuzViIqX95nxuMbQTFmAAAAAElFTkSuQmCC' |
        base64 -d > "$icon"
    {
        printf '[Desktop Entry]\nType=Application\nName=RustCalc\n'
        printf 'Comment=Native desktop calculator\nExec="%s"\nIcon=%s\n' "$target" "$icon"
        printf 'Terminal=false\nCategories=Utility;Calculator;\n'
    } > "$entry"
    printf '%s\n%s\n' "$entry" "$icon" >> "$created"
elif [ "$desktop" = 1 ] && [ "$os" = macos ]; then
    app="$HOME/Applications/RustCalc.app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    printf '%s' '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' |
        base64 -D > "$app/Contents/Resources/RustCalc.icns"
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
<key>CFBundleIconFile</key><string>RustCalc</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
EOF
    printf '%s\n' "$app" >> "$created"
fi

sort -u "$created" -o "$created"
write_marker "$version"

printf 'RustCalc %s installed to %s\n' "$version" "$target"
case ":$PATH:" in
    *":$bin_dir:"*) [ "$link" = 1 ] && printf 'Run: rustcalc\n' ;;
    *) [ "$link" = 1 ] && printf 'Add %s to your PATH to run rustcalc from any directory.\n' "$bin_dir" ;;
esac
printf 'Update: rustcalc update | Uninstall: sh "%s/uninstall.sh"\n' "$install_dir"
