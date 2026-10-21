#Requires -Version 5.1
<# .SYNOPSIS
Writes one SHA256SUMS for the Windows, Linux and macOS packages gathered in a
directory. Every expected payload must be present; nothing else is listed.
#>
[CmdletBinding()]
param([string]$Directory = 'dist', [string]$Version = '')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release-assets.ps1')
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $Version) {
    $cargo = Get-Content -LiteralPath (Join-Path $repoRoot 'Cargo.toml') -Raw
    $Version = [regex]::Match($cargo, '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
}
if ($Version -cnotmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') { throw 'Invalid version.' }
$Directory = [IO.Path]::GetFullPath($Directory)
$payloads = @(Get-RustCalcReleaseAssets $Version | Where-Object { $_ -cne 'SHA256SUMS' })
$lines = foreach ($name in $payloads) {
    $path = Join-Path $Directory $name
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing release payload: $name" }
    '{0}  {1}' -f (Microsoft.PowerShell.Utility\Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant(), $name
}
# LF line endings: the manifest is read by sh, PowerShell and the updater alike.
[IO.File]::WriteAllText((Join-Path $Directory 'SHA256SUMS'), (($lines -join "`n") + "`n"), (New-Object Text.UTF8Encoding($false)))
Write-Host "Assembled SHA256SUMS for $($payloads.Count) payloads of RustCalc v$Version"
