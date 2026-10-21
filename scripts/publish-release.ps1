#Requires -Version 5.1
<# .SYNOPSIS
Publishes a verified tested package, or safely resumes a matching draft release.
Existing assets are never overwritten. Requires an authenticated gh session.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [Parameter(Mandatory = $true)][string]$ExpectedCommit,
    [string]$Directory = 'dist',
    [string]$NotesFile = ''
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($Tag -cnotmatch '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') { throw 'Invalid stable release tag.' }
if ($ExpectedCommit -cnotmatch '^[0-9a-f]{40}$') { throw 'Invalid expected commit.' }
$repo = 'ersinkoc/rustcalc'
$version = $Tag.Substring(1)
$Directory = [IO.Path]::GetFullPath($Directory)
. (Join-Path $PSScriptRoot 'release-assets.ps1')
$names = @(Get-RustCalcReleaseAssets $version)
$hashes = @{}
foreach ($name in $names) {
    $path = Join-Path $Directory $name
    $hashes[$name] = (Microsoft.PowerShell.Utility\Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
}
$seen = @{}
foreach ($line in (Get-Content -LiteralPath (Join-Path $Directory 'SHA256SUMS'))) {
    if ($line -cnotmatch '^([0-9a-f]{64})  ([^\s]+)$') { throw 'Malformed package manifest.' }
    $digest = $Matches[1]; $name = $Matches[2]
    if ($names -cnotcontains $name -or $name -ceq 'SHA256SUMS' -or $seen.ContainsKey($name)) { throw "Unexpected manifest entry: $name" }
    if ($hashes[$name] -cne $digest) { throw "Package checksum mismatch: $name" }
    $seen[$name] = $true
}
if ($seen.Count -ne $names.Count - 1) { throw 'Incomplete package manifest.' }
$reference = gh api "repos/$repo/git/ref/tags/$Tag" | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $reference.object.type -cne 'tag') { throw 'The remote release tag must be annotated.' }
$tagObject = gh api "repos/$repo/git/tags/$($reference.object.sha)" | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $tagObject.object.type -cne 'commit' -or $tagObject.object.sha -cne $ExpectedCommit) { throw 'The remote tag does not point to the expected source commit.' }

$view = gh release view $Tag --repo $repo --json apiUrl,isDraft 2>$null
if ($LASTEXITCODE -ne 0) {
    $createArgs = @('release', 'create', $Tag, '--repo', $repo, '--verify-tag', '--draft', '--title', "RustCalc $Tag")
    if ($NotesFile) { $createArgs += @('--notes-file', [IO.Path]::GetFullPath($NotesFile)) }
    else { $createArgs += '--generate-notes' }
    & gh @createArgs
    if ($LASTEXITCODE -ne 0) { throw 'Draft release creation failed.' }
    $view = gh release view $Tag --repo $repo --json apiUrl,isDraft
    if ($LASTEXITCODE -ne 0) { throw 'Could not locate the new draft release.' }
}
$view = $view | ConvertFrom-Json
# Drafts may be absent from GET /releases/tags/{tag}. Resolve their REST API URL
# through gh's draft-aware lookup, then use the release ID endpoint instead.
$prefix = "https://api.github.com/repos/$repo/releases/"
if (-not $view.apiUrl.StartsWith($prefix, [StringComparison]::Ordinal) -or $view.apiUrl.Substring($prefix.Length) -notmatch '^[0-9]+$') { throw 'Unexpected release API URL.' }
$release = gh api $view.apiUrl | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $release.tag_name -cne $Tag) { throw 'Could not read release metadata.' }
foreach ($asset in $release.assets) {
    if ($names -cnotcontains $asset.name -or $asset.digest -cne ('sha256:' + $hashes[$asset.name])) { throw "Existing asset does not match the tested package: $($asset.name)" }
}
if (-not $release.draft) {
    if ($release.assets.Count -ne $names.Count) { throw 'Published release has unexpected assets.' }
    Write-Host "Published release already matches the tested package: $Tag"
    return
}
foreach ($name in $names) {
    if (@($release.assets | Where-Object { $_.name -ceq $name }).Count -eq 0) {
        gh release upload $Tag (Join-Path $Directory $name) --repo $repo
        if ($LASTEXITCODE -ne 0) { throw "Asset upload failed: $name" }
    }
}
$release = gh api $view.apiUrl | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $release.assets.Count -ne $names.Count) { throw 'Unexpected uploaded asset count.' }
foreach ($name in $names) {
    $uploaded = @($release.assets | Where-Object { $_.name -ceq $name })
    if ($uploaded.Count -ne 1 -or $uploaded[0].digest -cne ('sha256:' + $hashes[$name])) { throw "Uploaded digest mismatch: $name" }
    Write-Host "Verified uploaded digest: $name"
}
gh release edit $Tag --repo $repo --draft=false --latest
if ($LASTEXITCODE -ne 0) { throw 'Release publication failed.' }
$published = gh api $view.apiUrl | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $published.draft -or $published.tag_name -cne $Tag) { throw 'Published release state could not be confirmed.' }
Write-Host "Published https://github.com/$repo/releases/tag/$Tag"
