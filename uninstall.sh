#!/bin/sh
# Removes a RustCalc installation made by install.sh: the program folder's own
# files and the command link, menu entry or app it recorded. History and
# preferences are kept. Usage: sh uninstall.sh [INSTALL_DIR]
set -eu

fail() {
    printf 'RustCalc uninstaller: %s\n' "$1" >&2
    exit 1
}

install_dir="${1:-$(cd "$(dirname "$0")" && pwd)}"
install_dir="$(cd "$install_dir" 2>/dev/null && pwd)" || fail "No RustCalc installation marker found. No files were removed."
marker="$install_dir/install.json"
[ -f "$marker" ] || fail "No RustCalc installation marker found. No files were removed."
grep -q '"product": "RustCalc"' "$marker" || fail "Invalid RustCalc installation marker. No files were removed."
target="$install_dir/bin/rustcalc"
# RustCalc.app's launcher holds the target single-quoted, with ' written '\''.
quoted_target="$(printf '%s' "$target" | sed "s/'/'\\\\''/g")"

# Remove only recorded items that still belong to this installation.
if [ -f "$install_dir/created" ]; then
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        if [ -L "$path" ]; then
            [ "$(readlink "$path")" = "$target" ] && rm -f "$path"
        elif [ -d "$path" ]; then
            case "$path" in
                *.app) grep -qF "exec '$quoted_target'" "$path/Contents/MacOS/RustCalc" 2>/dev/null && rm -rf "$path" ;;
            esac
        elif [ -f "$path" ]; then
            case "$path" in
                *.desktop) grep -qF "$target" "$path" && rm -f "$path" ;;
                */rustcalc.png) rm -f "$path" ;;
            esac
        fi
    done < "$install_dir/created"
fi

# The rollback copy is kept by self-update beside the program.
rm -f "$target" "$install_dir/bin/rustcalc.previous"
rm -f "$install_dir/created" "$install_dir/install.json" "$install_dir/uninstall.sh"
# Never delete a folder recursively: only empty folders are removed.
rmdir "$install_dir/bin" 2>/dev/null || true
rmdir "$install_dir" 2>/dev/null || true
printf 'RustCalc uninstalled. Calculation history and preferences were kept.\n'
