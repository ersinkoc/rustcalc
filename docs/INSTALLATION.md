# Installation and removal

## Supported platform

Official binaries support x86-64 Windows. ARM64 and 32-bit Windows are not release
targets. A compatible OpenGL driver is required; running a binary needs no Rust
toolchain. The default installer needs no administrator access.

## Standard installation

Inspect and run `install.ps1` using Windows PowerShell 5.1+ or PowerShell 7+.

```powershell
& .\install.ps1
& .\install.ps1 -Version 0.1.0
& .\install.ps1 -InstallDir "$env:LOCALAPPDATA\Tools\RustCalc"
& .\install.ps1 -NoPath -NoShortcut
```

Default: `%LOCALAPPDATA%\Programs\RustCalc\bin\rustcalc.exe`. The installer pins
one release tag, downloads the binary/uninstaller/manifest, checks SHA-256 and
`--version`, then replaces the binary. Download/checksum/version failures leave
an existing installation unchanged. Close the app before rerunning the installer;
the app updater handles replacement while running.

Full PATH entries are compared, so repeated installs do not add duplicates.
The user and current process PATH are updated and Explorer is notified. A child
PowerShell process cannot modify its parent's environment; open a new terminal.
`-NoPath` and `-NoShortcut` skip those integrations.

## Offline installation

Download `rustcalc-windows-x86_64.exe`, `uninstall.ps1` and `SHA256SUMS` from the
same release, then run the repository/release installer:

```powershell
& .\install.ps1 -FromDirectory C:\Downloads\RustCalc -Version 0.1.0
```

Offline mode requires an explicit version and uses the same integrity/version
checks without requesting GitHub metadata.

## Portable use

Extract the ZIP or run the raw executable, optionally renamed to `rustcalc.exe`.
Compare `Get-FileHash -Algorithm SHA256` with the manifest first. Portable mode
still saves settings/history in the user's application-data directory.

## Uninstall

```powershell
& "$env:LOCALAPPDATA\Programs\RustCalc\uninstall.ps1"
```

For a custom install, run its own `uninstall.ps1`. The marker is checked; only
owned files, the exact PATH entry and owned shortcut are removed. Unrelated
files and calculation data are retained. No custom directory is recursively
deleted. Close running RustCalc windows first.

## Troubleshooting

- **No release / HTTP 404:** build from source or wait for the first release.
- **Scripts blocked:** inspect the script and follow your local policy. If
  permitted, use `Unblock-File .\install.ps1` or a one-time child process with
  `powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1`. Do not
  weaken a managed organization's policy.
- **Command not found:** open a new terminal and use `Get-Command rustcalc`.
  With `-NoPath`, run the executable by its absolute path.
- **File in use:** close other app windows. Check custom-directory permissions.
- **Checksum mismatch:** never bypass verification; fetch all files from the
  same release again and report a reproducible mismatch.
