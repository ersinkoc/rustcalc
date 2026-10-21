#!/bin/sh
# Integration checks for install.sh and uninstall.sh against local release
# fixtures built from a real binary. HOME points at a temporary folder, so the
# user's own files, PATH and menu entries are never touched.
#
#   sh scripts/test-install.sh [path/to/release/rustcalc]
set -eu

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
binary="${1:-$repo_root/target/release/rustcalc}"
[ -x "$binary" ] || { echo "Build first: cargo build --release --locked" >&2; exit 1; }
version="$(sed -n 's/^version = "\([0-9.]*\)"/\1/p' "$repo_root/Cargo.toml" | head -n 1)"
case "$(uname -s)-$(uname -m)" in
    Linux-*) asset=rustcalc-linux-x86_64; os=linux ;;
    Darwin-arm64) asset=rustcalc-macos-aarch64; os=macos ;;
    Darwin-*) asset=rustcalc-macos-x86_64; os=macos ;;
esac
if [ "$(uname -s)" = Darwin ] && [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" = 1 ]; then
    asset=rustcalc-macos-aarch64
fi

root="$(mktemp -d "${TMPDIR:-/tmp}/rustcalc-install-test-XXXXXX")"
trap 'rm -rf "$root"' EXIT
passed=0
failed=0
check() {
    if [ "$1" = 0 ]; then
        passed=$((passed + 1)); printf 'PASS: %s\n' "$2"
    else
        failed=$((failed + 1)); printf 'FAIL: %s\n' "$2"
    fi
}
sha() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d ' ' -f 1; else shasum -a 256 "$1" | cut -d ' ' -f 1; fi
}
fixture() {
    rm -rf "$root/fixture"; mkdir -p "$root/fixture"
    cp "$binary" "$root/fixture/$asset"
    cp "$repo_root/uninstall.sh" "$root/fixture/uninstall.sh"
    printf '%s  %s\n%s  %s\n' "$(sha "$root/fixture/$asset")" "$asset" \
        "$(sha "$root/fixture/uninstall.sh")" uninstall.sh > "$root/fixture/SHA256SUMS"
}
expect_failure() {
    # expect_failure "<message fragment>" "<description>" -- installer args...
    fragment="$1"; description="$2"; shift 3
    if output="$(sh "$repo_root/install.sh" "$@" 2>&1)"; then
        check 1 "$description (installer succeeded)"
    else
        printf '%s' "$output" | grep -q "$fragment"
        check $? "$description (observed: $output)"
    fi
}

export HOME="$root/home"
mkdir -p "$HOME"
original_path="$PATH"
install_dir="$root/installed app 'quote \$ dollar"
bin_dir="$HOME/.local/bin"
fixture

sh "$repo_root/install.sh" --from-directory "$root/fixture" --version "$version" --install-dir "$install_dir" >/dev/null
[ "$(sha "$install_dir/bin/rustcalc")" = "$(sha "$binary")" ]
check $? "installer places the verified binary"
[ "$("$bin_dir/rustcalc" --version)" = "rustcalc $version" ]
check $? "the rustcalc link runs the installed version"
grep -q '"product": "RustCalc"' "$install_dir/install.json"
check $? "installer writes an ownership marker"
if [ "$os" = linux ]; then
    grep -qF "$install_dir/bin/rustcalc" "$HOME/.local/share/applications/rustcalc.desktop" &&
        [ -s "$HOME/.local/share/icons/hicolor/64x64/apps/rustcalc.png" ]
    check $? "a menu entry and icon point at the installation"
else
    grep -qF "$install_dir/bin/rustcalc" "$HOME/Applications/RustCalc.app/Contents/MacOS/RustCalc"
    check $? "RustCalc.app launches the installation"
fi
sh "$repo_root/install.sh" --from-directory "$root/fixture" --version "$version" --install-dir "$install_dir" >/dev/null
[ "$("$bin_dir/rustcalc" --version)" = "rustcalc $version" ]
check $? "reinstalling over an existing installation works"
[ "$PATH" = "$original_path" ] && [ ! -e "$HOME/.profile" ] && [ ! -e "$HOME/.bashrc" ] && [ ! -e "$HOME/.zshrc" ]
check $? "PATH and shell startup files are never changed"

before="$(sha "$install_dir/bin/rustcalc")"
printf 'tampered' >> "$root/fixture/$asset"
expect_failure "SHA-256 verification failed" "a tampered binary is rejected" -- \
    --from-directory "$root/fixture" --version "$version" --install-dir "$install_dir"
[ "$(sha "$install_dir/bin/rustcalc")" = "$before" ]
check $? "a rejected download leaves the installation unchanged"
fixture
expect_failure "version does not match" "a binary of another version is rejected" -- \
    --from-directory "$root/fixture" --version 99.0.0 --install-dir "$install_dir"
head -n 1 "$root/fixture/SHA256SUMS" >> "$root/fixture/SHA256SUMS"
expect_failure "exactly one checksum" "duplicate checksum entries are rejected" -- \
    --from-directory "$root/fixture" --version "$version" --install-dir "$install_dir"
fixture
mkdir -p "$root/occupied"; echo user > "$root/occupied/notes.txt"
expect_failure "not empty" "an unrelated non-empty folder is refused" -- \
    --from-directory "$root/fixture" --version "$version" --install-dir "$root/occupied"
expect_failure "requires --version" "offline installs need an explicit version" -- \
    --from-directory "$root/fixture" --install-dir "$install_dir"

echo keep > "$install_dir/keep-me.txt"
cp "$install_dir/bin/rustcalc" "$install_dir/bin/rustcalc.previous"
mkdir -p "$HOME/.local/share/rustcalc"; echo history > "$HOME/.local/share/rustcalc/app.ron"
sh "$install_dir/uninstall.sh" >/dev/null
[ ! -e "$install_dir/bin/rustcalc" ] && [ ! -e "$install_dir/bin/rustcalc.previous" ] && [ ! -L "$bin_dir/rustcalc" ]
check $? "uninstall removes the program, its rollback copy and the command link"
if [ "$os" = linux ]; then
    [ ! -e "$HOME/.local/share/applications/rustcalc.desktop" ]
else
    [ ! -e "$HOME/Applications/RustCalc.app" ]
fi
check $? "uninstall removes the menu entry or app"
[ -f "$install_dir/keep-me.txt" ] && [ -f "$HOME/.local/share/rustcalc/app.ron" ]
check $? "uninstall keeps unrelated files, history and preferences"
if output="$(sh "$repo_root/uninstall.sh" "$root/occupied" 2>&1)"; then
    check 1 "uninstall refuses unmarked folders"
else
    printf '%s' "$output" | grep -q "No RustCalc installation marker" && [ -f "$root/occupied/notes.txt" ]
    check $? "uninstall refuses unmarked folders"
fi

printf 'Installer checks: %s passed, %s failed.\n' "$passed" "$failed"
[ "$failed" = 0 ]
