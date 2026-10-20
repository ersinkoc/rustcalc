#Requires -Version 5.1
<# .SYNOPSIS
Removes RustCalc's installed files, PATH entry and Start menu shortcut.
Calculation history and preferences are kept.
#>
[CmdletBinding()]
param([string]$InstallDir = '', [switch]$NoPath, [switch]$NoShortcut)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not $InstallDir) { $InstallDir = $PSScriptRoot }
$InstallDir = [IO.Path]::GetFullPath($InstallDir).TrimEnd('\')
$marker = Join-Path $InstallDir 'install.json'
if (-not (Test-Path -LiteralPath $marker)) { throw 'No RustCalc installation marker found. No files were removed.' }
$installation = Get-Content -LiteralPath $marker -Raw | ConvertFrom-Json
if ($installation.product -cne 'RustCalc' -or [IO.Path]::GetFullPath($installation.installDir).TrimEnd('\') -ine $InstallDir) { throw 'Invalid RustCalc installation marker. No files were removed.' }
$binDir = Join-Path $InstallDir 'bin'
$target = Join-Path $binDir 'rustcalc.exe'
if (Test-Path -LiteralPath $target) {
    try { Remove-Item -LiteralPath $target -Force } catch { throw 'Close all RustCalc windows before uninstalling. Your installation was kept.' }
}
if (-not $NoPath) {
    foreach ($scope in @('User', 'Process')) {
        $existing = [Environment]::GetEnvironmentVariable('Path', $scope)
        $entries = @($existing -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and $_.Trim().TrimEnd('\') -ine $binDir.TrimEnd('\') })
        [Environment]::SetEnvironmentVariable('Path', ($entries -join ';'), $scope)
    }
}
if (-not $NoShortcut) {
    $shortcutPath = Join-Path ([Environment]::GetFolderPath('Programs')) 'RustCalc.lnk'
    if (Test-Path -LiteralPath $shortcutPath) {
        $shell = New-Object -ComObject WScript.Shell
        if ($shell.CreateShortcut($shortcutPath).TargetPath -ieq $target) { Remove-Item -LiteralPath $shortcutPath -Force }
    }
}
# Delete only known owned files. Never recursively remove a custom directory.
foreach ($name in @('install.json', 'uninstall.ps1')) {
    $owned = Join-Path $InstallDir $name
    if (Test-Path -LiteralPath $owned) { Remove-Item -LiteralPath $owned -Force }
}
if ((Test-Path -LiteralPath $binDir) -and @(Get-ChildItem -LiteralPath $binDir -Force).Count -eq 0) { Remove-Item -LiteralPath $binDir }
if ((Test-Path -LiteralPath $InstallDir) -and @(Get-ChildItem -LiteralPath $InstallDir -Force).Count -eq 0) { Remove-Item -LiteralPath $InstallDir }
Write-Host 'RustCalc uninstalled. Calculation history and preferences were kept.'
