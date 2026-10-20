#Requires -Version 5.1
<#
.SYNOPSIS
Installs a SHA-256-verified RustCalc Windows x64 release for the current user.
.EXAMPLE
& .\install.ps1 -Version 0.1.0
.EXAMPLE
& .\install.ps1 -FromDirectory C:\Downloads\RustCalc -Version 0.1.0 -NoPath -NoShortcut
#>
[CmdletBinding()]
param(
    [string]$Version = '',
    [string]$InstallDir = '',
    [string]$FromDirectory = '',
    [switch]$NoPath,
    [switch]$NoShortcut
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Merge-RustCalcPath([string]$Existing, [string]$Directory) {
    $entries = @($Existing -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $match = @($entries | Where-Object { $_.Trim().TrimEnd('\') -ieq $Directory.TrimEnd('\') })
    if ($match.Count -eq 0) { $entries += $Directory }
    return ($entries -join ';')
}
function Get-RustCalcChecksum([string]$Manifest, [string]$Name) {
    $foundChecksums = @()
    foreach ($line in ($Manifest -split '\r?\n')) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        if ($line -notmatch '^([a-fA-F0-9]{64})\s+\*?([^\s]+)$') { throw 'Malformed SHA256SUMS entry.' }
        if ($Matches[2] -ceq $Name) { $foundChecksums += $Matches[1].ToLowerInvariant() }
    }
    if ($foundChecksums.Count -ne 1) { throw "Expected exactly one checksum for $Name." }
    return $foundChecksums[0]
}
function Get-RustCalcBinaryVersion([string]$Path) {
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $Path
    $info.Arguments = '--version'
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::Start($info)
    if (-not $process.WaitForExit(10000)) { $process.Kill(); throw 'RustCalc version verification timed out.' }
    $output = $process.StandardOutput.ReadToEnd().Trim()
    if ($process.ExitCode -ne 0 -or $output -notmatch '^rustcalc ([0-9]+\.[0-9]+\.[0-9]+)$') { throw 'The downloaded executable did not identify itself as RustCalc.' }
    return $Matches[1]
}

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'This installer supports Windows only.' }
$architecture = $env:PROCESSOR_ARCHITECTURE
if ($env:PROCESSOR_ARCHITEW6432) { $architecture = $env:PROCESSOR_ARCHITEW6432 }
if ($architecture -ne 'AMD64') { throw 'The current release supports Windows x64 only.' }
if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\RustCalc' }
$InstallDir = [IO.Path]::GetFullPath($InstallDir)
if ($InstallDir.TrimEnd('\') -eq [IO.Path]::GetPathRoot($InstallDir).TrimEnd('\')) { throw 'Choose an application directory, not a drive root.' }
$existingMarker = Join-Path $InstallDir 'install.json'
if (Test-Path -LiteralPath $existingMarker) {
    $existingInstall = Get-Content -LiteralPath $existingMarker -Raw | ConvertFrom-Json
    if ($existingInstall.product -cne 'RustCalc' -or [IO.Path]::GetFullPath($existingInstall.installDir).TrimEnd('\') -ine $InstallDir.TrimEnd('\')) { throw 'The directory has a conflicting installation marker.' }
} elseif ((Test-Path -LiteralPath $InstallDir) -and @(Get-ChildItem -LiteralPath $InstallDir -Force).Count -ne 0) {
    throw 'The installation directory is not empty and is not a RustCalc installation.'
}
$binDir = Join-Path $InstallDir 'bin'
$target = Join-Path $binDir 'rustcalc.exe'
$headers = @{ 'User-Agent' = 'RustCalc-Installer'; 'Accept' = 'application/vnd.github+json' }
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
if ($Version -and $Version -notmatch '^v?(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') { throw 'Version must be a stable version such as 0.1.0.' }
if ($FromDirectory -and -not $Version) { throw '-FromDirectory requires an explicit -Version.' }
if (-not $Version) {
    try { $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/ersinkoc/rustcalc/releases/latest' -Headers $headers -TimeoutSec 30 }
    catch { throw 'Could not find a stable RustCalc release. Check https://github.com/ersinkoc/rustcalc/releases or use -Version.' }
    if ($release.draft -or $release.prerelease -or $release.tag_name -notmatch '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') { throw 'The latest release is not a valid stable release.' }
    $Version = $release.tag_name
}
$Version = $Version.TrimStart('v')
$releaseBase = "https://github.com/ersinkoc/rustcalc/releases/download/v$Version"
$stage = Join-Path ([IO.Path]::GetTempPath()) ('rustcalc-install-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage | Out-Null
try {
    foreach ($name in @('SHA256SUMS', 'rustcalc-windows-x86_64.exe', 'uninstall.ps1')) {
        $destination = Join-Path $stage $name
        if ($FromDirectory) { Copy-Item -LiteralPath (Join-Path ([IO.Path]::GetFullPath($FromDirectory)) $name) -Destination $destination }
        else { Invoke-WebRequest -Uri "$releaseBase/$name" -OutFile $destination -Headers @{ 'User-Agent' = 'RustCalc-Installer' } -TimeoutSec 120 -UseBasicParsing }
    }
    $manifest = Get-Content -LiteralPath (Join-Path $stage 'SHA256SUMS') -Raw
    foreach ($name in @('rustcalc-windows-x86_64.exe', 'uninstall.ps1')) {
        $expected = Get-RustCalcChecksum $manifest $name
        $actual = (Microsoft.PowerShell.Utility\Get-FileHash -LiteralPath (Join-Path $stage $name) -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($expected -cne $actual) { throw "SHA-256 verification failed for $name. The installed application was not changed." }
    }
    $downloaded = Join-Path $stage 'rustcalc-windows-x86_64.exe'
    if ((Get-Item -LiteralPath $downloaded).Length -gt 64MB) { throw 'The release executable exceeds the size limit.' }
    if ((Get-RustCalcBinaryVersion $downloaded) -cne $Version) { throw 'The executable version does not match the requested release.' }
    New-Item -ItemType Directory -Path $binDir -Force | Out-Null
    # File.Replace retains the original if replacement fails (e.g. a running GUI).
    $pending = Join-Path $binDir ('rustcalc-' + [guid]::NewGuid().ToString('N') + '.pending.exe')
    $backup = Join-Path $binDir ('rustcalc-' + [guid]::NewGuid().ToString('N') + '.backup.exe')
    Copy-Item -LiteralPath $downloaded -Destination $pending
    $installedVerified = $false
    try {
        if (Test-Path -LiteralPath $target) { [IO.File]::Replace($pending, $target, $backup) }
        else { [IO.File]::Move($pending, $target) }
        try {
            if ((Get-RustCalcBinaryVersion $target) -cne $Version) { throw 'Installed executable verification failed.' }
            $installedVerified = $true
        } catch {
            if (Test-Path -LiteralPath $backup) { [IO.File]::Replace($backup, $target, $null) }
            else { Remove-Item -LiteralPath $target -Force }
            throw
        }
    } catch { throw "Installation failed. Close RustCalc and retry. $($_.Exception.Message)" }
    finally {
        if (Test-Path -LiteralPath $pending) { Remove-Item -LiteralPath $pending -Force }
        if ($installedVerified -and (Test-Path -LiteralPath $backup)) { Remove-Item -LiteralPath $backup -Force }
    }
    Copy-Item -LiteralPath (Join-Path $stage 'uninstall.ps1') -Destination (Join-Path $InstallDir 'uninstall.ps1') -Force
    @{ product = 'RustCalc'; installDir = $InstallDir; version = $Version } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $InstallDir 'install.json') -Encoding UTF8
    if (-not $NoPath) {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        [Environment]::SetEnvironmentVariable('Path', (Merge-RustCalcPath $userPath $binDir), 'User')
        $env:Path = Merge-RustCalcPath $env:Path $binDir
        # Notify Explorer so newly opened terminals inherit the new user PATH.
        if (-not ('RustCalcInstaller.NativeMethods' -as [type])) {
            Add-Type -Namespace RustCalcInstaller -Name NativeMethods -MemberDefinition '[System.Runtime.InteropServices.DllImport("user32.dll", CharSet=System.Runtime.InteropServices.CharSet.Unicode)] public static extern System.IntPtr SendMessageTimeout(System.IntPtr hWnd, uint Msg, System.UIntPtr wParam, string lParam, uint flags, uint timeout, out System.UIntPtr result);'
        }
        $messageResult = [UIntPtr]::Zero
        [RustCalcInstaller.NativeMethods]::SendMessageTimeout([IntPtr]0xffff, 0x1A, [UIntPtr]::Zero, 'Environment', 2, 2000, [ref]$messageResult) | Out-Null
    }
    if (-not $NoShortcut) {
        $shell = New-Object -ComObject WScript.Shell
        $shortcut = $shell.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Programs')) 'RustCalc.lnk'))
        $shortcut.TargetPath = $target
        $shortcut.WorkingDirectory = $InstallDir
        $shortcut.Description = 'RustCalc — a native desktop calculator'
        $shortcut.Save()
    }
    Write-Host "RustCalc $Version installed to $target"
    Write-Host 'Run rustcalc from any directory. Open a new terminal if using a separate PowerShell process.'
    Write-Host 'Update: rustcalc update | Uninstall: & <InstallDir>\uninstall.ps1'
} finally {
    $resolvedStage = [IO.Path]::GetFullPath($stage)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if ($resolvedStage.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and ([IO.Path]::GetFileName($resolvedStage)).StartsWith('rustcalc-install-')) {
        Remove-Item -LiteralPath $resolvedStage -Recurse -Force
    }
}
