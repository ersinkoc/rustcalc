# Dot-source to get the exact asset names of a release. Every publishing and
# verification script uses this one list. Releases before 0.5.0 were
# Windows-only and keep their original five assets.
function Get-RustCalcReleaseAssets([string]$Version) {
    $windows = @('rustcalc-windows-x86_64.exe', "rustcalc-v$Version-windows-x86_64.zip", 'install.ps1', 'uninstall.ps1')
    if ([version]$Version -lt [version]'0.5.0') { return $windows + @('SHA256SUMS') }
    $unix = @()
    foreach ($platform in @('linux-x86_64', 'macos-aarch64', 'macos-x86_64')) {
        $unix += @("rustcalc-$platform", "rustcalc-v$Version-$platform.tar.gz")
    }
    return $windows + $unix + @('install.sh', 'uninstall.sh', 'SHA256SUMS')
}
