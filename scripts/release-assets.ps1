# Dot-source to get the exact asset names of a release. Every publishing and
# verification script uses this one list. Releases before 0.5.0 were
# Windows-only and keep their original five assets; ARM64 Windows and Linux
# builds start with 0.7.0.
function Get-RustCalcReleaseAssets([string]$Version) {
    $windows = @('rustcalc-windows-x86_64.exe', "rustcalc-v$Version-windows-x86_64.zip")
    if ([version]$Version -ge [version]'0.7.0') {
        $windows += @('rustcalc-windows-aarch64.exe', "rustcalc-v$Version-windows-aarch64.zip")
    }
    $windows += @('install.ps1', 'uninstall.ps1')
    if ([version]$Version -lt [version]'0.5.0') { return $windows + @('SHA256SUMS') }
    $platforms = @('linux-x86_64', 'macos-aarch64', 'macos-x86_64')
    if ([version]$Version -ge [version]'0.7.0') { $platforms = @('linux-x86_64', 'linux-aarch64', 'macos-aarch64', 'macos-x86_64') }
    $unix = @()
    foreach ($platform in $platforms) {
        $unix += @("rustcalc-$platform", "rustcalc-v$Version-$platform.tar.gz")
    }
    return $windows + $unix + @('install.sh', 'uninstall.sh', 'SHA256SUMS')
}
