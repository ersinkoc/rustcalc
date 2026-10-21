#Requires -Version 5.1
[CmdletBinding()]
param([string]$Binary = '')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
# Windows PowerShell 5.1 does not initialize PSScriptRoot during parameter defaults.
if (-not $Binary) { $Binary = Join-Path $repoRoot 'target\release\rustcalc.exe' }
$Binary = [IO.Path]::GetFullPath($Binary)
if (-not (Test-Path -LiteralPath $Binary)) { throw 'Build the executable first.' }
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('rustcalc-install-test-' + [guid]::NewGuid().ToString('N'))
$originalPath = $env:Path
$originalLocation = (Get-Location).Path
$script:passed = 0
function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw "FAIL: $Message" }
    $script:passed++
    Write-Host "PASS: $Message"
}
function Assert-Throws([scriptblock]$Action, [string]$Pattern, [string]$Message) {
    $failure = $null
    try { & $Action } catch { $failure = $_.Exception.Message }
    Assert-True ($null -ne $failure -and $failure -match $Pattern) "$Message (observed: $failure)"
}
try {
    New-Item -ItemType Directory -Path $testRoot | Out-Null
    $fixture = Join-Path $testRoot 'release'
    & (Join-Path $PSScriptRoot 'package.ps1') -Binary $Binary -OutputDirectory $fixture
    $versionLine = (& $Binary --version | Out-String).Trim()
    Assert-True ($versionLine -match '^rustcalc [0-9]+\.[0-9]+\.[0-9]+$') 'packaged executable reports a valid version'
    $version = $versionLine.Substring(9)
    $installDir = Join-Path $testRoot "space and 'quote `$ dollar"
    $installer = Join-Path $repoRoot 'install.ps1'
    & $installer -Version $version -FromDirectory $fixture -InstallDir $installDir -NoPath -NoShortcut
    $installed = Join-Path $installDir 'bin\rustcalc.exe'
    Assert-True (Test-Path -LiteralPath $installed) 'offline installation creates the canonical executable'
    Assert-True ((Get-FileHash -LiteralPath $installed).Hash -ceq (Get-FileHash -LiteralPath $Binary).Hash) 'installed executable matches the verified source'
    Assert-True ($env:Path -ceq $originalPath) '-NoPath does not modify process PATH'
    & $installer -Version $version -FromDirectory $fixture -InstallDir $installDir -NoPath -NoShortcut
    Assert-True (Test-Path -LiteralPath (Join-Path $installDir 'install.json')) 'reinstallation remains usable and writes an ownership marker'

    # Exercise canonical command lookup from outside the checkout.
    $env:Path = (Join-Path $installDir 'bin') + ';' + $originalPath
    Set-Location -LiteralPath ([IO.Path]::GetTempPath())
    $actual = (& rustcalc --version | Out-String).Trim()
    Assert-True ($actual -ceq "rustcalc $version") 'rustcalc resolves and runs from outside the repository'
    $env:Path = $originalPath

    # Load pure helpers from the real standalone installer without executing it.
    $tokens = $null; $parseErrors = $null
    $ast = [Management.Automation.Language.Parser]::ParseFile($installer, [ref]$tokens, [ref]$parseErrors)
    Assert-True ($parseErrors.Count -eq 0) 'installer parses without syntax errors'
    $helper = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Merge-RustCalcPath' }, $true)
    Invoke-Expression $helper.Extent.Text
    Assert-True ((Merge-RustCalcPath 'C:\Tools\RustCalcExtra;C:\Other' 'C:\Tools\RustCalc') -ceq 'C:\Tools\RustCalcExtra;C:\Other;C:\Tools\RustCalc') 'PATH comparison handles prefix-colliding directories'
    Assert-True ((Merge-RustCalcPath 'C:\TOOLS\RUSTCALC\;C:\Other' 'C:\Tools\RustCalc') -ceq 'C:\TOOLS\RUSTCALC\;C:\Other') 'PATH comparison is idempotent across case and trailing slash'

    $before = (Get-FileHash -LiteralPath $installed).Hash
    $fixtureBinary = Join-Path $fixture 'rustcalc-windows-x86_64.exe'
    [IO.File]::AppendAllText($fixtureBinary, 'tampered')
    Assert-Throws { & $installer -Version $version -FromDirectory $fixture -InstallDir $installDir -NoPath -NoShortcut } 'SHA-256' 'tampered binary is rejected'
    Assert-True ((Get-FileHash -LiteralPath $installed).Hash -ceq $before) 'tampering leaves the installed application unchanged'
    Copy-Item -LiteralPath $Binary -Destination $fixtureBinary -Force

    $wrongVersion = if ($version -eq '99.0.0') { '98.0.0' } else { '99.0.0' }
    Assert-Throws { & $installer -Version $wrongVersion -FromDirectory $fixture -InstallDir $installDir -NoPath -NoShortcut } 'version does not match' 'wrong executable version is rejected before installation'
    $manifestPath = Join-Path $fixture 'SHA256SUMS'
    $manifest = Get-Content -LiteralPath $manifestPath -Raw
    $binaryEntry = @($manifest -split '\r?\n' | Where-Object { $_ -match 'rustcalc-windows-x86_64\.exe$' })[0]
    [IO.File]::AppendAllText($manifestPath, "$binaryEntry`n")
    Assert-Throws { & $installer -Version $version -FromDirectory $fixture -InstallDir $installDir -NoPath -NoShortcut } 'exactly one checksum' 'duplicate checksum entries are rejected'

    $unrelated = Join-Path $installDir 'keep-me.txt'
    Set-Content -LiteralPath $unrelated -Value 'user-owned file'
    $previous = Join-Path (Split-Path -Parent $installed) 'rustcalc.previous.exe'
    Copy-Item -LiteralPath $installed -Destination $previous
    & (Join-Path $installDir 'uninstall.ps1') -NoPath -NoShortcut
    Assert-True (-not (Test-Path -LiteralPath $installed)) 'uninstall removes the installed executable'
    Assert-True (-not (Test-Path -LiteralPath $previous)) 'uninstall removes the rollback copy kept by self-update'
    Assert-True (Test-Path -LiteralPath $unrelated) 'uninstall preserves unrelated files in the installation directory'
    Assert-True ($env:Path -ceq $originalPath) 'test and -NoPath uninstall preserve process PATH'
    Assert-Throws { & (Join-Path $repoRoot 'uninstall.ps1') -InstallDir $testRoot -NoPath -NoShortcut } 'No RustCalc installation marker' 'uninstall refuses unmarked directories'
    Write-Host "Installer checks: $script:passed passed, 0 failed."
} finally {
    $env:Path = $originalPath
    Set-Location -LiteralPath $originalLocation
    $resolved = [IO.Path]::GetFullPath($testRoot)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if ($resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and ([IO.Path]::GetFileName($resolved)).StartsWith('rustcalc-install-test-') -and (Test-Path -LiteralPath $resolved)) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
