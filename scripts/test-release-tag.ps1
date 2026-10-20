#Requires -Version 5.1
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$verifier = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot 'verify-release-tag.ps1'))
$fixtureRoot = Join-Path ([IO.Path]::GetTempPath()) ('rustcalc-tag-test-' + [guid]::NewGuid().ToString('N'))
$originalLocation = (Get-Location).Path
$script:passed = 0
function Git-Checked {
    $value = & git @args
    if ($LASTEXITCODE -ne 0) { throw "Fixture git command failed: $($args -join ' ')" }
    return $value
}
function Assert-Tag([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw "FAIL: $Message" }
    $script:passed++
    Write-Host "PASS: $Message"
}
function Refuses([scriptblock]$Action, [string]$Pattern) {
    $failure = ''
    try { & $Action } catch { $failure = $_.Exception.Message }
    Assert-Tag ($failure -match $Pattern) "rejects $Pattern"
}
try {
    New-Item -ItemType Directory -Path $fixtureRoot | Out-Null
    $remote = Join-Path $fixtureRoot 'remote.git'
    $repository = Join-Path $fixtureRoot 'checkout'
    Git-Checked init --bare --initial-branch=main $remote | Out-Null
    Git-Checked init --initial-branch=main $repository | Out-Null
    Set-Location -LiteralPath $repository
    Git-Checked config user.name 'RustCalc tests'
    Git-Checked config user.email 'tests@example.invalid'
    Set-Content -LiteralPath Cargo.toml -Value @('[package]', 'name = "fixture"', 'version = "1.2.3"')
    Git-Checked add Cargo.toml
    Git-Checked -c commit.gpgsign=false commit -m fixture | Out-Null
    $commit = Git-Checked rev-parse HEAD
    Git-Checked remote add origin $remote
    Git-Checked push -u origin main | Out-Null
    Git-Checked -c tag.gpgsign=false tag -a v1.2.3 -m fixture
    Git-Checked push origin v1.2.3 | Out-Null
    & $verifier -Tag v1.2.3 -ExpectedCommit $commit
    Assert-Tag ((Git-Checked cat-file -t v1.2.3) -ceq 'tag') 'accepts the original annotated release tag'
    # Reproduce actions/checkout replacing the local ref with its peeled commit.
    Git-Checked update-ref refs/tags/v1.2.3 $commit
    Assert-Tag ((Git-Checked cat-file -t v1.2.3) -ceq 'commit') 'fixture reproduces a peeled local tag'
    & $verifier -Tag v1.2.3 -ExpectedCommit $commit
    Assert-Tag ((Git-Checked cat-file -t v1.2.3) -ceq 'tag') 'restores and verifies the authoritative annotated remote tag'
    Refuses { & $verifier -Tag v1.2.3 -ExpectedCommit ('0' * 40) } 'expected release commit'
    Refuses { & $verifier -Tag v9.9.9 -ExpectedCommit $commit } 'Cargo.toml'
    # A genuinely lightweight remote tag must remain invalid.
    Git-Checked tag -d v1.2.3 | Out-Null
    Git-Checked push origin :refs/tags/v1.2.3 | Out-Null
    Git-Checked -c tag.gpgsign=false tag v1.2.3
    Git-Checked push origin v1.2.3 | Out-Null
    Refuses { & $verifier -Tag v1.2.3 -ExpectedCommit $commit } 'must be annotated'
    Write-Host "Release tag checks: $script:passed passed, 0 failed."
} finally {
    Set-Location -LiteralPath $originalLocation
    $resolved = [IO.Path]::GetFullPath($fixtureRoot)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if ($resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and ([IO.Path]::GetFileName($resolved)).StartsWith('rustcalc-tag-test-') -and (Test-Path -LiteralPath $resolved)) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
