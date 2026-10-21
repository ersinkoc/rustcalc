# Installation and removal

## Supported platforms

| Platform | Release files | Installer |
| --- | --- | --- |
| Windows 10/11, x64 | `rustcalc-windows-x86_64.exe`, ZIP | `install.ps1` |
| Linux, x64 (glibc 2.35+, e.g. Ubuntu 22.04, Debian 12, Fedora 36 or newer) | `rustcalc-linux-x86_64`, tar.gz | `install.sh` |
| macOS 11+, Apple silicon | `rustcalc-macos-aarch64`, tar.gz with `RustCalc.app` | `install.sh` |
| macOS 11+, Intel | `rustcalc-macos-x86_64`, tar.gz with `RustCalc.app` | `install.sh` |

All platforms need an OpenGL-capable graphics driver. Linux needs an X11 or
Wayland desktop. Running RustCalc needs no Rust toolchain, and the installers
need no administrator access. Other systems (Windows ARM64, Linux ARM64, BSD)
can build from source with `cargo build --release --locked`.

## Windows

Inspect and run `install.ps1` using Windows PowerShell 5.1+ or PowerShell 7+.

```powershell
& .\install.ps1
& .\install.ps1 -Version 0.6.0
& .\install.ps1 -InstallDir "$env:LOCALAPPDATA\Tools\RustCalc"
& .\install.ps1 -NoPath -NoShortcut
```

Default: `%LOCALAPPDATA%\Programs\RustCalc\bin\rustcalc.exe`. The installer pins
one release tag, downloads the binary, uninstaller and manifest, checks SHA-256
and `--version`, then replaces the binary. Download, checksum or version
failures leave an existing installation unchanged. Close the app before
rerunning the installer; the app updater handles replacement while running.

Full PATH entries are compared, so repeated installs do not add duplicates.
The user and current process PATH are updated and Explorer is notified. A child
PowerShell process cannot modify its parent's environment; open a new terminal.
`-NoPath` and `-NoShortcut` skip those integrations.

Offline: download `rustcalc-windows-x86_64.exe`, `uninstall.ps1` and
`SHA256SUMS` from the same release, then run
`& .\install.ps1 -FromDirectory C:\Downloads\RustCalc -Version 0.6.0`.

Uninstall with `& "$env:LOCALAPPDATA\Programs\RustCalc\uninstall.ps1"` (or the
`uninstall.ps1` of a custom installation). Only owned files, the exact PATH
entry and the owned shortcut are removed.

## Linux and macOS

Download and inspect the installer, then run it with `sh`:

```sh
curl -fsSLO https://raw.githubusercontent.com/ersinkoc/rustcalc/main/install.sh
# Review install.sh before running it.
sh install.sh
rustcalc
```

| Option | Effect |
| --- | --- |
| `--version 0.6.0` | Install a specific release instead of the latest |
| `--install-dir DIR` | Program folder (default `~/.local/opt/rustcalc`) |
| `--bin-dir DIR` | Folder for the `rustcalc` command link (default `~/.local/bin`) |
| `--from-directory DIR` | Offline install from downloaded files; needs `--version` |
| `--no-link` | Skip the command link |
| `--no-desktop` | Skip the menu entry (Linux) or `~/Applications/RustCalc.app` (macOS) |

The installer picks the binary for your system, downloads it with `uninstall.sh`
and `SHA256SUMS` from one release, checks the SHA-256 and the binary's
`--version`, and only then replaces `~/.local/opt/rustcalc/bin/rustcalc`
atomically. It links `~/.local/bin/rustcalc` to it, and adds a menu entry with
an icon on Linux or a small `RustCalc.app` launcher in `~/Applications` on
macOS. It never edits shell startup files: if `~/.local/bin` is not on your
PATH it says so, and you can add it yourself.

Offline: download `rustcalc-<platform>`, `uninstall.sh` and `SHA256SUMS` from the
same release and run `sh install.sh --from-directory ~/Downloads --version 0.6.0`.

Uninstall with `sh ~/.local/opt/rustcalc/uninstall.sh`. It checks the
installation marker, removes the program, its rollback copy and only the link,
menu entry or app the installer recorded, and deletes folders only when empty.

### Portable use and RustCalc.app

The tar.gz contains the `rustcalc` binary, documentation and both scripts; on
macOS it also contains `RustCalc.app`, which you can drag to Applications.
Compare `sha256sum` (Linux) or `shasum -a 256` (macOS) with `SHA256SUMS` first.

The binaries are not signed. Files fetched by `install.sh` or the updater are
not quarantined, but macOS Gatekeeper blocks a browser-downloaded app the first
time: Control-click `RustCalc.app` and choose **Open**, or run
`xattr -dr com.apple.quarantine RustCalc.app` after checking the checksum.

## Where data is kept

History, memory, settings and your formulas are stored separately from the
program and survive updates and uninstalling:

| Platform | State file |
| --- | --- |
| Windows | `%APPDATA%\RustCalc\data\app.ron` |
| Linux | `~/.local/share/rustcalc/app.ron` |
| macOS | `~/Library/Application Support/RustCalc/app.ron` |

## Troubleshooting

- **Scripts blocked (Windows):** inspect the script and follow your local
  policy. If permitted, use `Unblock-File .\install.ps1` or
  `powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1`.
- **Command not found:** open a new terminal. On Linux/macOS add
  `~/.local/bin` to PATH, or run `~/.local/opt/rustcalc/bin/rustcalc` directly.
- **Linux window does not open:** make sure an OpenGL driver (Mesa) and
  `libxkbcommon` are installed; both are present on standard desktops.
- **File in use (Windows):** close other app windows and retry.
- **Checksum mismatch:** never bypass verification; fetch all files from the
  same release again and report a reproducible mismatch.
