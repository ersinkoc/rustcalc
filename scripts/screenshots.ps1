#Requires -Version 5.1
<# .SYNOPSIS
Regenerates the README screenshots in docs/images from the release build with
demo data. Saved history and settings are not read or changed.
#>
[CmdletBinding()]
param([string]$Executable = 'target/release/rustcalc.exe')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$exe = [IO.Path]::GetFullPath((Join-Path $repoRoot $Executable))
if (-not (Test-Path -LiteralPath $exe)) { throw "Build first: cargo build --release --locked ($exe not found)" }
$images = Join-Path $repoRoot 'docs/images'
$shots = [ordered]@{
    'calculator-en-dark.png'    = @('--lang', 'en')
    'calculator-en-light.png'   = @('--lang', 'en', '--light')
    'calculator-tr-dark.png'    = @('--lang', 'tr')
    'calculator-tr-compact.png' = @('--lang', 'tr', '--compact')
    'converter-en.png'          = @('--lang', 'en', '--converter')
    'programmer-tr.png'         = @('--lang', 'tr', '--programmer')
    'graph-en-light.png'        = @('--lang', 'en', '--graph', '--light')
    'formulas-en-dark.png'      = @('--lang', 'en', '--formulas')
    'settings-tr-dark.png'      = @('--lang', 'tr', '--settings')
}
foreach ($name in $shots.Keys) {
    $out = Join-Path $images $name
    $arguments = @('--screenshot', $out, '--demo') + $shots[$name]
    $process = Start-Process -FilePath $exe -ArgumentList $arguments -PassThru
    if (-not $process.WaitForExit(30000)) { $process.Kill(); throw "Timed out: $name" }
    if (-not (Test-Path -LiteralPath $out)) { throw "No screenshot written: $name" }
    Write-Host "Wrote docs/images/$name"
}
