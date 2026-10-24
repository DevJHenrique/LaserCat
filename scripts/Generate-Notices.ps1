# SPDX-License-Identifier: AGPL-3.0-only
param([Parameter(Mandatory=$true)][string]$OutputPath)
$ErrorActionPreference = 'Stop'
$metadataJson = & cargo metadata --locked --offline --format-version 1
if ($LASTEXITCODE -ne 0) { throw 'Could not read dependency metadata for license notices.' }
$metadata = $metadataJson | ConvertFrom-Json
$noticeText = [System.Text.StringBuilder]::new()
[void]$noticeText.AppendLine('LaserCat - Third-party notices')
[void]$noticeText.AppendLine('LaserCat is licensed under AGPL-3.0-only. Dependencies retain the individual licenses below.')
[void]$noticeText.AppendLine('Dependency source archives are available from the versioned crates.io links below.')
foreach ($dependency in ($metadata.packages | Where-Object source | Sort-Object name, version)) {
    [void]$noticeText.AppendLine("`n===== $($dependency.name) $($dependency.version) =====")
    [void]$noticeText.AppendLine("Declared license: $($dependency.license)")
    [void]$noticeText.AppendLine("Source: https://crates.io/crates/$($dependency.name)/$($dependency.version)")
    [void]$noticeText.AppendLine("Upstream: $($dependency.repository)")
    $dependencyRoot = Split-Path -Parent $dependency.manifest_path
    # Embedded fonts ship their licenses as OFL.txt/UFL.txt or as a .txt named after the font file.
    $licenseFiles = Get-ChildItem -LiteralPath $dependencyRoot -File -Recurse | Where-Object {
        $isLicenseName = $_.Name -match '(?i)(licen[sc]e|copying|notice)' -or $_.BaseName -match '(?i)^(OFL|UFL)$'
        $isFontLicense = $_.Extension -eq '.txt' -and ((Test-Path -LiteralPath ([System.IO.Path]::ChangeExtension($_.FullName, '.ttf'))) -or (Test-Path -LiteralPath ([System.IO.Path]::ChangeExtension($_.FullName, '.otf'))))
        ($isLicenseName -or $isFontLicense) -and $_.Extension -notin '.rs','.c','.h','.toml','.json','.lock','.ttf','.otf','.png','.jpg'
    }
    foreach ($licenseFile in $licenseFiles) {
        if ($licenseFile.Length -lt 300000) {
            [void]$noticeText.AppendLine("`n--- $($licenseFile.Name) ---")
            [void]$noticeText.AppendLine([System.IO.File]::ReadAllText($licenseFile.FullName))
        }
    }
}
[System.IO.File]::WriteAllText($OutputPath, $noticeText.ToString(), [System.Text.UTF8Encoding]::new($false))
