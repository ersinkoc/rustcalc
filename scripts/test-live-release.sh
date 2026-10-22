#!/bin/sh
# Verifies a published release on Linux or macOS from the public download URLs:
# installs it with that release's install.sh into a temporary HOME, runs the
# command, checks for updates and uninstalls. With an older version as second
# argument it installs that one and exercises the real self-update and
# rollback (both releases must have assets for this platform: 0.5.0 or later
# on Linux x64 and macOS, 0.7.0 or later on Linux ARM64).
#
#   sh scripts/test-live-release.sh 0.7.0 [0.6.0]
set -eu

version="$1"
from="${2:-}"
start="${from:-$version}"
repo="ersinkoc/rustcalc"
root="$(mktemp -d "${TMPDIR:-/tmp}/rustcalc-live-XXXXXX")"
trap 'rm -rf "$root"' EXIT
export HOME="$root/home"
mkdir -p "$HOME"
app="$root/app"
bin="$app/bin/rustcalc"
passed=0
pass() { passed=$((passed + 1)); printf 'PASS: %s\n' "$1"; }
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }

curl -fsSL --proto '=https' -o "$root/install.sh" "https://github.com/$repo/releases/download/v$start/install.sh" ||
    fail "could not download install.sh for v$start"
sh "$root/install.sh" --version "$start" --install-dir "$app" --no-desktop >/dev/null || fail "public installer failed"
pass "public installer: $start"
[ "$("$HOME/.local/bin/rustcalc" --version)" = "rustcalc $start" ] || fail "command link does not run $start"
pass "command link runs rustcalc $start"

check="$("$bin" update --check)"
if [ -z "$from" ]; then
    printf '%s' "$check" | grep -q "RustCalc is up to date." || fail "latest version should be up to date: $check"
    pass "latest installed version reports up to date"
else
    printf '%s' "$check" | grep -q "Update available: $start -> $version" || fail "update not offered: $check"
    pass "update check offers $start -> $version"
    "$bin" update >/dev/null || fail "self-update failed"
    [ "$("$bin" --version)" = "rustcalc $version" ] || fail "self-update did not install $version"
    pass "real self-update: $start -> $version"
    [ -x "$app/bin/rustcalc.previous" ] || fail "no rollback copy was kept"
    "$bin" update --rollback >/dev/null || fail "rollback failed"
    [ "$("$bin" --version)" = "rustcalc $start" ] || fail "rollback did not restore $start"
    pass "rollback restores $start"
    "$bin" update --rollback >/dev/null || fail "redo failed"
    [ "$("$bin" --version)" = "rustcalc $version" ] || fail "second rollback did not return to $version"
    pass "running rollback again returns to $version"
fi

sh "$app/uninstall.sh" >/dev/null || fail "uninstall failed"
[ ! -e "$bin" ] && [ ! -L "$HOME/.local/bin/rustcalc" ] || fail "uninstall left files behind"
pass "public uninstaller removes the installation"
printf 'Live release proof (%s): passed; %s checks.\n' "$(uname -s)" "$passed"
