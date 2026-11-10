#Requires -Version 5.1
[CmdletBinding()]
param([string]$Directory = '', [string]$PreviousDirectory = '', [string]$PreviousVersion = '')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $Directory) { $Directory = Join-Path $repo 'dist' }
$registry = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\RustCalc.Desktop_is1'
if (Test-Path -LiteralPath $registry) { throw 'Run this test on a machine without a registered RustCalc setup installation.' }
$setups = @(Get-ChildItem -LiteralPath $Directory -File -Filter '*-setup.exe')
if ($setups.Count -ne 1) { throw 'Expected one native setup package for this host.' }
$setup = $setups[0].FullName
if ($setups[0].Name -cnotmatch '^rustcalc-v([0-9]+\.[0-9]+\.[0-9]+)-windows-(x86_64|aarch64)-setup\.exe$') { throw 'Unexpected setup filename.' }
$version = $Matches[1]
$binary = Join-Path $Directory "rustcalc-windows-$($Matches[2]).exe"
$root = Join-Path ([IO.Path]::GetTempPath()) ('rustcalc-setup-test-' + [guid]::NewGuid().ToString('N'))
$install = Join-Path $root 'application with spaces'
$target = Join-Path $install 'bin\rustcalc.exe'
$originalPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$state = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'RustCalc\data\app.ron'
$before = if (Test-Path -LiteralPath $state) { (Get-FileHash -LiteralPath $state).Hash } else { '' }
$flags = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-', ('/DIR="' + $install + '"'), '/TASKS=""')

function Run([string]$Executable, [string[]]$Options) {
    $process = Start-Process -FilePath $Executable -ArgumentList $Options -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode) { throw "$Executable exited with code $($process.ExitCode)." }
}

function Assert([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
    Write-Host "PASS: $Message"
}

New-Item -ItemType Directory -Path $root | Out-Null
try {
    # Start with the existing scripted layout and exercise adoption into Apps.
    if ($PreviousDirectory) {
        if (-not $PreviousVersion) { throw 'PreviousDirectory requires PreviousVersion.' }
        $source = $PreviousDirectory
        $sourceVersion = $PreviousVersion
    } else {
        $source = $Directory
        $sourceVersion = $version
    }
    & (Join-Path $repo 'install.ps1') -Version $sourceVersion -FromDirectory $source -InstallDir $install -NoPath -NoShortcut
    $legacyBinary = Join-Path $root 'legacy.exe'
    Copy-Item -LiteralPath $target -Destination $legacyBinary
    $unrelated = Join-Path $install 'keep-me.txt'
    Set-Content -LiteralPath $unrelated 'Preserve this file.'
    Run $setup ($flags + ('/LOG="' + (Join-Path $root 'setup.log') + '"'))
    $registered = Get-ItemProperty -LiteralPath $registry
    Assert ($registered.DisplayVersion -ceq $version) 'Installed apps registration identifies the packaged version'
    Assert ($registered.InstallLocation.TrimEnd('\') -ieq $install) 'Setup used the requested per-user location'
    Assert ((Get-FileHash -LiteralPath $target).Hash -ceq (Get-FileHash -LiteralPath $binary).Hash) 'Installed binary matches the release payload'
    Assert ((& $target --version | Out-String).Trim() -ceq "rustcalc $version") 'Installed binary answers its own version probe'
    Assert (-not (Test-Path -LiteralPath (Join-Path $install 'install.json'))) 'Setup adopted the script installation'
    Run $setup $flags
    Assert (Test-Path -LiteralPath $target -PathType Leaf) 'Reinstall works on the registered directory'
    if ($PreviousDirectory) {
        $previous = Join-Path $install 'bin\rustcalc.previous.exe'
        Copy-Item -LiteralPath $legacyBinary -Destination $previous
        Assert ((& $target update --rollback | Out-String).Trim() -like "Restored RustCalc $PreviousVersion.*") 'The installed application rolls back to the previous release'
        Assert ((Get-FileHash -LiteralPath $target).Hash -ceq (Get-FileHash -LiteralPath $legacyBinary).Hash) 'Rollback restores the exact previous release binary'
        Assert ((& $target update --rollback | Out-String).Trim() -like "Restored RustCalc $version.*") 'Rollback can be undone from the previous release'
        Assert ((Get-FileHash -LiteralPath $target).Hash -ceq (Get-FileHash -LiteralPath $binary).Hash) 'Redo restores the exact GPUI build'
    }
    $uninstaller = $registered.UninstallString.Trim('"')
    Run $uninstaller @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    Assert (-not (Test-Path -LiteralPath $target) -and -not (Test-Path -LiteralPath $registry)) 'Uninstall removes the binary and Apps entry'
    Assert ((Get-Content -LiteralPath $unrelated -Raw).Trim() -ceq 'Preserve this file.') 'Uninstall preserves other files in the install directory'
    Assert ([Environment]::GetEnvironmentVariable('Path', 'User') -ceq $originalPath) 'Setup without the PATH task preserves user PATH'
    $after = if (Test-Path -LiteralPath $state) { (Get-FileHash -LiteralPath $state).Hash } else { '' }
    Assert ($before -ceq $after) 'Setup and uninstall preserve the notebook'
} catch {
    $log = Join-Path $root 'setup.log'
    if (Test-Path -LiteralPath $log) { Get-Content -LiteralPath $log -Tail 45 | Write-Host }
    throw
} finally {
    if (Test-Path -LiteralPath $registry) {
        $registered = Get-ItemProperty -LiteralPath $registry
        if ($registered.InstallLocation.TrimEnd('\') -ieq $install) {
            $uninstaller = $registered.UninstallString.Trim('"')
            if (Test-Path -LiteralPath $uninstaller) { Run $uninstaller @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART') }
        }
    }
    $resolved = (Resolve-Path -LiteralPath $root).ProviderPath
    $temp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if ($resolved.StartsWith($temp, [StringComparison]::OrdinalIgnoreCase) -and [IO.Path]::GetFileName($resolved).StartsWith('rustcalc-setup-test-')) {
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
