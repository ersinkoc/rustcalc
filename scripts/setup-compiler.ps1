#Requires -Version 5.1
[CmdletBinding()]
param([string]$Directory = '')
$ErrorActionPreference = 'Stop'
if (-not $Directory) { $Directory = Join-Path $PSScriptRoot '../artifacts/inno-6.7.4' }
$Directory = [IO.Path]::GetFullPath($Directory)
$compilerDir = Join-Path $Directory 'compiler'
$compiler = Join-Path $compilerDir 'ISCC.exe'
$bootstrap = Join-Path $Directory 'innosetup-6.7.4.exe'
$sha256 = 'a549929319f9120716677c685535ca4671e4b937b5fdc87e2585f32d8eb30e8c'
if (-not (Test-Path -LiteralPath $compiler -PathType Leaf)) {
    New-Item -ItemType Directory -Path $Directory -Force | Out-Null
    if (-not (Test-Path -LiteralPath $bootstrap -PathType Leaf)) {
        Invoke-WebRequest -UseBasicParsing -Uri 'https://github.com/jrsoftware/issrc/releases/download/is-6_7_4/innosetup-6.7.4.exe' -OutFile $bootstrap
    }
    if ((Get-FileHash -LiteralPath $bootstrap -Algorithm SHA256).Hash -ine $sha256) { throw 'Inno Setup compiler checksum mismatch.' }
    $process = Start-Process -FilePath $bootstrap -ArgumentList @(
        '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/NOICONS', '/PORTABLE', ('/DIR="' + $compilerDir + '"')
    ) -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $compiler -PathType Leaf)) {
        throw "Inno Setup compiler installation failed ($($process.ExitCode))."
    }
}
$compiler
