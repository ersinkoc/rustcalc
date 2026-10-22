#Requires -Version 5.1
<#
.SYNOPSIS
Verifies a published GitHub release, installs it from the public download URLs,
and optionally exercises a real self-update from an older published release.
Uses an isolated temporary installation. Never changes persistent PATH.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$UpgradeFrom = '',
    [string]$ExpectedCommit = '',
    [string]$ReportPath = ''
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release-assets.ps1')
foreach ($value in @($Version, $UpgradeFrom)) {
    if ($value -and $value -notmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') { throw 'Use stable version numbers without the v prefix.' }
}
if (-not $ReportPath) { $ReportPath = Join-Path $PSScriptRoot "..\artifacts\live-release-v$Version.json" }
$ReportPath = [IO.Path]::GetFullPath($ReportPath)
$taskRoot = Join-Path ([IO.Path]::GetTempPath()) ('rustcalc-live-release-' + [guid]::NewGuid().ToString('N'))
$originalPath = $env:Path
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$originalLocation = (Get-Location).Path
$stateFile = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'RustCalc\data\app.ron'
$stateHash = ''
if (Test-Path -LiteralPath $stateFile) { $stateHash = (Microsoft.PowerShell.Utility\Get-FileHash -LiteralPath $stateFile -Algorithm SHA256).Hash }
$script:checks = New-Object 'System.Collections.Generic.List[object]'
$headers = @{ 'User-Agent' = 'RustCalc-Release-Proof'; 'Accept' = 'application/vnd.github+json' }
$report = [ordered]@{ version = $Version; upgradeFrom = $UpgradeFrom; status = 'running'; checks = $script:checks }
function Record-Check([string]$Name, [string]$Detail = '') {
    $script:checks.Add([ordered]@{ check = $Name; detail = $Detail; status = 'passed' })
    Write-Host "PASS: $Name $Detail"
}
function Require([bool]$Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function Hash-File([string]$Path) { return (Microsoft.PowerShell.Utility\Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
function Invoke-Binary([string]$Path, [string]$Arguments, [int]$Timeout = 15000) {
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = $Path
    $info.Arguments = $Arguments
    $info.WorkingDirectory = [IO.Path]::GetTempPath()
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($info)
    if (-not $process.WaitForExit($Timeout)) { $process.Kill(); throw "Timed out: rustcalc $Arguments" }
    return [pscustomobject]@{ Code = $process.ExitCode; Out = $process.StandardOutput.ReadToEnd().Trim(); Error = $process.StandardError.ReadToEnd().Trim() }
}
# Windows architectures with a published executable for a release.
function Get-WindowsArchitectures([string]$ReleaseVersion) {
    if ([version]$ReleaseVersion -ge [version]'0.7.0') { return @('x86_64', 'aarch64') }
    return @('x86_64')
}
# The executable install.ps1 picks on this machine, as it decides it.
function Get-WindowsAsset([string]$ReleaseVersion) {
    $native = (Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment' -Name PROCESSOR_ARCHITECTURE).PROCESSOR_ARCHITECTURE
    if ($native -eq 'ARM64' -and (Get-WindowsArchitectures $ReleaseVersion) -contains 'aarch64') { return 'rustcalc-windows-aarch64.exe' }
    return 'rustcalc-windows-x86_64.exe'
}
function Get-ReleaseFiles([string]$ReleaseVersion) {
    $tag = "v$ReleaseVersion"
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/ersinkoc/rustcalc/releases/tags/$tag" -Headers $headers -TimeoutSec 30
    Require (-not $release.draft -and -not $release.prerelease -and $release.tag_name -ceq $tag) 'Release is not a published stable release.'
    $expected = @(Get-RustCalcReleaseAssets $ReleaseVersion)
    Require ($release.assets.Count -eq $expected.Count) 'Unexpected release asset count.'
    $directory = Join-Path $taskRoot $tag
    New-Item -ItemType Directory -Path $directory | Out-Null
    foreach ($name in $expected) {
        $assets = @($release.assets | Where-Object { $_.name -ceq $name })
        Require ($assets.Count -eq 1) "Missing or duplicate asset: $name"
        $asset = $assets[0]
        $url = "https://github.com/ersinkoc/rustcalc/releases/download/$tag/$name"
        Require ($asset.browser_download_url -ceq $url) "Untrusted asset URL: $name"
        Require ($asset.size -gt 0 -and $asset.size -le 64MB) "Invalid asset size: $name"
        $file = Join-Path $directory $name
        Invoke-WebRequest -Uri $url -OutFile $file -Headers @{ 'User-Agent' = 'RustCalc-Release-Proof' } -TimeoutSec 120 -UseBasicParsing
        Require ((Get-Item -LiteralPath $file).Length -eq $asset.size) "Asset size mismatch: $name"
        Require ($asset.digest -ceq ('sha256:' + (Hash-File $file))) "GitHub digest mismatch: $name"
        Record-Check "GitHub digest: $tag/$name"
    }
    $entries = @{}
    foreach ($line in (Get-Content -LiteralPath (Join-Path $directory 'SHA256SUMS'))) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        Require ($line -cmatch '^([0-9a-f]{64})  ([^\s]+)$') 'Malformed checksum manifest.'
        $name = $Matches[2]; $digest = $Matches[1]
        Require (-not $entries.ContainsKey($name) -and $expected -ccontains $name -and $name -cne 'SHA256SUMS') "Unexpected checksum entry: $name"
        $entries[$name] = $digest
        Require ((Hash-File (Join-Path $directory $name)) -ceq $digest) "Manifest mismatch: $name"
    }
    Require ($entries.Count -eq $expected.Count - 1) 'Checksum manifest must cover every downloadable payload.'
    Record-Check "Complete checksum manifest: $tag"
    $reference = Invoke-RestMethod -Uri "https://api.github.com/repos/ersinkoc/rustcalc/git/ref/tags/$tag" -Headers $headers -TimeoutSec 30
    Require ($reference.object.type -ceq 'tag') 'Release tag is not annotated.'
    $tagObject = Invoke-RestMethod -Uri "https://api.github.com/repos/ersinkoc/rustcalc/git/tags/$($reference.object.sha)" -Headers $headers -TimeoutSec 30
    Require ($tagObject.object.type -ceq 'commit') 'Annotated tag does not point to a commit.'
    if ($ReleaseVersion -ceq $Version -and $ExpectedCommit) { Require ($tagObject.object.sha -ceq $ExpectedCommit) 'Release tag points to the wrong commit.' }
    Record-Check "Annotated tag: $tag" $tagObject.object.sha
    # Only this machine's own executable can run; every architecture's ZIP is checked.
    $probe = Invoke-Binary (Join-Path $directory (Get-WindowsAsset $ReleaseVersion)) '--version'
    Require ($probe.Code -eq 0 -and $probe.Out -ceq "rustcalc $ReleaseVersion") 'Release executable version does not match the tag.'
    Record-Check "Release executable version: $ReleaseVersion" (Get-WindowsAsset $ReleaseVersion)
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    foreach ($architecture in (Get-WindowsArchitectures $ReleaseVersion)) {
        $exe = "rustcalc-windows-$architecture.exe"
        $binary = Join-Path $directory $exe
        $archive = [IO.Compression.ZipFile]::OpenRead((Join-Path $directory "rustcalc-v$ReleaseVersion-windows-$architecture.zip"))
        try {
            $zipExpected = @($exe, 'README.md', 'LICENSE', 'CHANGELOG.md', 'install.ps1', 'uninstall.ps1')
            Require ($archive.Entries.Count -eq $zipExpected.Count) "Unexpected portable ZIP contents: $architecture"
            foreach ($name in $zipExpected) { Require (@($archive.Entries | Where-Object { $_.FullName -ceq $name }).Count -eq 1) "Missing or duplicate ZIP file: $name" }
            $entry = $archive.GetEntry($exe).Open()
            $hasher = [Security.Cryptography.SHA256]::Create()
            try { $zipHash = [BitConverter]::ToString($hasher.ComputeHash($entry)).Replace('-', '').ToLowerInvariant() }
            finally { $entry.Dispose(); $hasher.Dispose() }
            Require ($zipHash -ceq (Hash-File $binary)) "ZIP executable differs from the raw release executable: $architecture"
        } finally { $archive.Dispose() }
        Record-Check "Portable ZIP contents and executable: $tag $architecture"
    }
    return $directory
}
try {
    New-Item -ItemType Directory -Path $taskRoot | Out-Null
    $latest = Invoke-RestMethod -Uri 'https://api.github.com/repos/ersinkoc/rustcalc/releases/latest' -Headers $headers -TimeoutSec 30
    Require ($latest.tag_name -ceq "v$Version") 'The expected release is not latest; refusing an ambiguous live update test.'
    $targetFiles = Get-ReleaseFiles $Version
    $startVersion = $Version
    $startFiles = $targetFiles
    if ($UpgradeFrom) { $startVersion = $UpgradeFrom; $startFiles = Get-ReleaseFiles $UpgradeFrom }
    $installDirectory = Join-Path $taskRoot "installed app 'quote `$ dollar"
    & (Join-Path $startFiles 'install.ps1') -Version $startVersion -InstallDir $installDirectory -NoPath -NoShortcut
    $installed = Join-Path $installDirectory 'bin\rustcalc.exe'
    Require ((Hash-File $installed) -ceq (Hash-File (Join-Path $startFiles (Get-WindowsAsset $startVersion)))) 'Public installer produced the wrong binary.'
    Record-Check "Public network installer: $startVersion"
    $env:Path = (Join-Path $installDirectory 'bin') + ';' + $originalPath
    Set-Location -LiteralPath ([IO.Path]::GetTempPath())
    # Get-Command lists every match; PowerShell runs the first on PATH.
    $canonical = (Get-Command rustcalc -CommandType Application | Select-Object -First 1).Source
    Require ([IO.Path]::GetFullPath($canonical) -ieq [IO.Path]::GetFullPath($installed)) 'Canonical command resolved to a different installation.'
    $probe = Invoke-Binary $canonical '--version'
    Require ($probe.Code -eq 0 -and $probe.Out -ceq "rustcalc $startVersion") 'Canonical command failed outside the checkout.'
    Record-Check 'Canonical command outside the checkout' $probe.Out
    $before = Hash-File $installed
    $check = Invoke-Binary $installed 'update --check' 120000
    Require ($check.Code -eq 0) "Update check failed: $($check.Error)"
    Require ((Hash-File $installed) -ceq $before) 'Check-only modified the executable.'
    Record-Check 'Live check-only leaves the executable unchanged' $check.Out
    if ($UpgradeFrom) {
        Require ($check.Out.Contains("$UpgradeFrom -> $Version")) 'The old binary did not find the expected upgrade.'
        $update = Invoke-Binary $installed 'update' 240000
        Require ($update.Code -eq 0) "Self-update failed: $($update.Error)"
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        $upgraded = $false
        do {
            try { $probe = Invoke-Binary $installed '--version'; $upgraded = $probe.Code -eq 0 -and $probe.Out -ceq "rustcalc $Version" } catch { $upgraded = $false }
            if (-not $upgraded) { Start-Sleep -Milliseconds 100 }
        } while (-not $upgraded -and [DateTime]::UtcNow -lt $deadline)
        Require $upgraded 'The updated executable did not become available.'
        Require ((Hash-File $installed) -ceq (Hash-File (Join-Path $targetFiles (Get-WindowsAsset $Version)))) 'Updated executable differs from the public release.'
        Record-Check "Real self-update: $UpgradeFrom -> $Version" $update.Out
    }
    $check = Invoke-Binary $installed 'update --check' 120000
    Require ($check.Code -eq 0 -and $check.Out.Contains('up to date')) 'The final version did not report up-to-date.'
    Record-Check 'Latest installed version reports up-to-date'
    & (Join-Path $installDirectory 'uninstall.ps1') -NoPath -NoShortcut
    Require (-not (Test-Path -LiteralPath $installed)) 'The public uninstaller left the executable behind.'
    Record-Check 'Public uninstaller removes the isolated installation'
    Require ([Environment]::GetEnvironmentVariable('Path', 'User') -ceq $userPath) 'The proof changed persistent user PATH.'
    Record-Check 'Persistent user PATH preserved'
    if ($stateHash) { Require ((Microsoft.PowerShell.Utility\Get-FileHash -LiteralPath $stateFile -Algorithm SHA256).Hash -ceq $stateHash) 'History or preferences changed during installation/update/removal.' }
    else { Require (-not (Test-Path -LiteralPath $stateFile)) 'The CLI proof unexpectedly created GUI state.' }
    Record-Check 'Existing calculation history and preferences preserved'
    $report.status = 'passed'
} catch {
    $report.status = 'failed'
    $report['error'] = $_.Exception.Message
    throw
} finally {
    $env:Path = $originalPath
    Set-Location -LiteralPath $originalLocation
    New-Item -ItemType Directory -Path ([IO.Path]::GetDirectoryName($ReportPath)) -Force | Out-Null
    $report | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $ReportPath -Encoding UTF8
    $resolved = [IO.Path]::GetFullPath($taskRoot)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if ($resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and ([IO.Path]::GetFileName($resolved)).StartsWith('rustcalc-live-release-') -and (Test-Path -LiteralPath $resolved)) { Remove-Item -LiteralPath $resolved -Recurse -Force }
    Write-Host "Live release proof: $($report.status); $($script:checks.Count) checks. Report: $ReportPath"
}
