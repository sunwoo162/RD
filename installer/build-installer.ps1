param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^v[0-9]+\.[0-9]+\.[0-9]+(?:-beta\.[0-9]+)?$')]
    [string]$Version,
    [string]$OutputDir = (Join-Path $PSScriptRoot 'output')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$makensis = @(
    (Join-Path ([Environment]::GetFolderPath('ProgramFilesX86')) 'NSIS\makensis.exe'),
    (Join-Path $env:ProgramFiles 'NSIS\makensis.exe')
) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $makensis) {
    throw 'NSIS is required to build the installer (makensis.exe was not found).'
}

$working = Join-Path $env:TEMP ('rd-installer-' + [guid]::NewGuid().ToString('N'))
$portable = Join-Path $working 'portable'
$archive = Join-Path $working "RD-$Version-Windows-x64.zip"
$checksum = Join-Path $working 'SHA256SUMS.txt'
$repository = 'sunwoo162/RD'

try {
    New-Item -ItemType Directory -Force -Path $working, $portable, $OutputDir | Out-Null
    & gh release download $Version --repo $repository --pattern "RD-$Version-Windows-x64.zip" --dir $working
    if ($LASTEXITCODE -ne 0) { throw 'Failed to download the exact portable release ZIP.' }
    & gh release download $Version --repo $repository --pattern 'SHA256SUMS.txt' --dir $working
    if ($LASTEXITCODE -ne 0) { throw 'Failed to download the release checksum.' }

    $line = Get-Content -LiteralPath $checksum -Encoding utf8 | Where-Object {
        $_ -match ('(?i)^([0-9a-f]{64})\s+\*?RD-' + [regex]::Escape($Version) + '-Windows-x64\.zip$')
    } | Select-Object -First 1
    if (-not $line) { throw 'The release has no matching SHA-256 checksum line.' }
    $expected = [regex]::Match($line, '(?i)^[0-9a-f]{64}').Value.ToLowerInvariant()
    $actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($expected -ne $actual) { throw 'SHA-256 mismatch: refusing to package a modified ZIP.' }

    Expand-Archive -LiteralPath $archive -DestinationPath $portable -ErrorAction Stop
    foreach ($required in @('rd-manager.exe','rd-agent.exe','rd-tray.exe','README.md','LICENSE')) {
        if (-not (Test-Path -LiteralPath (Join-Path $portable $required))) {
            throw "Missing file in portable archive: $required"
        }
    }

    $target = Join-Path ([System.IO.Path]::GetFullPath($OutputDir)) "RepoDock-Setup-$Version.exe"
    $nsi = Join-Path $PSScriptRoot 'RepoDock.nsi'
    & $makensis "/DAPP_VERSION=$Version" "/DSOURCE_DIR=$portable" "/DOUTPUT_EXE=$target" $nsi
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $target)) {
        throw "NSIS failed to build the installer for $Version"
    }
    Write-Output "INSTALLER_READY=$target"
    Write-Output "INSTALLER_SHA256=$((Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash)"
} finally {
    if (Test-Path -LiteralPath $working) {
        Remove-Item -LiteralPath $working -Recurse -Force
    }
}
