#Requires -Version 5.1
[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Tag, [Parameter(Mandatory = $true)][string]$ExpectedCommit)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($Tag -cnotmatch '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') { throw 'Invalid stable release tag.' }
if ($ExpectedCommit -cnotmatch '^[0-9a-f]{40}$') { throw 'Invalid expected commit.' }
$version = [regex]::Match((Get-Content -LiteralPath Cargo.toml -Raw), '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
if ($Tag -cne "v$version") { throw 'Tag does not match Cargo.toml.' }
$headCommit = git rev-parse HEAD
if ($LASTEXITCODE -ne 0 -or $headCommit -cne $ExpectedCommit) { throw 'Checked-out commit does not match the expected release commit.' }
# actions/checkout can rewrite an annotated local tag to the event's peeled
# commit. Refetch the authoritative tag object before checking its type/target.
$tagRef = "refs/tags/$Tag"
git fetch --no-tags --force origin "${tagRef}:${tagRef}"
if ($LASTEXITCODE -ne 0) { throw 'Could not fetch the authoritative release tag.' }
$tagType = git cat-file -t $tagRef
if ($LASTEXITCODE -ne 0 -or $tagType -cne 'tag') { throw 'Release tags must be annotated.' }
$tagCommit = git rev-parse "${tagRef}^{}"
if ($LASTEXITCODE -ne 0 -or $tagCommit -cne $ExpectedCommit) { throw 'Annotated tag points to the wrong commit.' }
git merge-base --is-ancestor $ExpectedCommit origin/main
if ($LASTEXITCODE -ne 0) { throw 'Release commit must belong to main.' }
Write-Host "Verified annotated tag $Tag -> $ExpectedCommit"
