#Requires -Version 5.1
[CmdletBinding()]
param([string]$OutputDirectory = '', [string]$Binary = '')
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $repoRoot 'dist' }
if (-not $Binary) { $Binary = Join-Path $repoRoot 'target\release\rustcalc.exe' }
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
$Binary = [IO.Path]::GetFullPath($Binary)
$cargo = Get-Content -LiteralPath (Join-Path $repoRoot 'Cargo.toml') -Raw
$version = [regex]::Match($cargo, '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
if (-not $version) { throw 'Could not determine the package version.' }
if (-not (Test-Path -LiteralPath $Binary)) { throw 'Build first: cargo build --release --locked' }
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$asset = Join-Path $OutputDirectory 'rustcalc-windows-x86_64.exe'
Copy-Item -LiteralPath $Binary -Destination $asset -Force
foreach ($name in @('README.md', 'LICENSE', 'CHANGELOG.md', 'install.ps1', 'uninstall.ps1')) { Copy-Item -LiteralPath (Join-Path $repoRoot $name) -Destination (Join-Path $OutputDirectory $name) -Force }
$zip = Join-Path $OutputDirectory "rustcalc-v$version-windows-x86_64.zip"
Compress-Archive -LiteralPath $asset,(Join-Path $OutputDirectory 'README.md'),(Join-Path $OutputDirectory 'LICENSE'),(Join-Path $OutputDirectory 'CHANGELOG.md'),(Join-Path $OutputDirectory 'install.ps1'),(Join-Path $OutputDirectory 'uninstall.ps1') -DestinationPath $zip -Force
$lines = foreach ($file in @($asset, $zip, (Join-Path $OutputDirectory 'install.ps1'), (Join-Path $OutputDirectory 'uninstall.ps1'))) { '{0}  {1}' -f (Microsoft.PowerShell.Utility\Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant(),[IO.Path]::GetFileName($file) }
[IO.File]::WriteAllLines((Join-Path $OutputDirectory 'SHA256SUMS'), $lines, (New-Object Text.UTF8Encoding($false)))
Write-Host "Packaged RustCalc v$version in $OutputDirectory"
