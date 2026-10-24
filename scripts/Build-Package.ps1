# SPDX-License-Identifier: AGPL-3.0-only
param([switch]$SkipBuild, [switch]$Offline)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $projectRoot
try {
    if (-not $SkipBuild) {
        $buildArguments = @('build','--release','--locked')
        if ($Offline) { $buildArguments += '--offline' }
        & cargo @buildArguments
        if ($LASTEXITCODE -ne 0) { throw 'Rust compilation failed.' }
    }
    $manifestText = [System.IO.File]::ReadAllText((Join-Path $projectRoot 'Cargo.toml'))
    $packageVersion = [regex]::Match($manifestText, '(?m)^version = "([^"]+)"').Groups[1].Value
    if (-not $packageVersion) { throw 'Package version not found.' }
    $distribution = Join-Path $projectRoot "dist\$packageVersion"
    New-Item -ItemType Directory -Path $distribution -Force | Out-Null
    $binary = Join-Path $projectRoot 'target\release\lasercat.exe'
    Copy-Item -LiteralPath $binary -Destination (Join-Path $distribution 'lasercat.exe') -Force
    Copy-Item -LiteralPath (Join-Path $projectRoot 'README.md'), (Join-Path $projectRoot 'LICENSE') -Destination $distribution -Force
    Copy-Item -LiteralPath (Join-Path $projectRoot 'calibration') -Destination $distribution -Recurse -Force
    $rootDocuments = @('ARCHITECTURE.md','CREDITS.md','CONTRIBUTING.md','CHANGELOG.md','SECURITY.md','AGENTS.md','CLAUDE.md','agent.md','NOTICE')
    foreach ($document in $rootDocuments) { Copy-Item -LiteralPath (Join-Path $projectRoot $document) -Destination $distribution -Force }
    Copy-Item -LiteralPath (Join-Path $projectRoot 'docs') -Destination $distribution -Recurse -Force
    Copy-Item -LiteralPath (Join-Path $projectRoot 'LICENSES') -Destination $distribution -Recurse -Force
    $sourceArchiveName = "LaserCat-$packageVersion-source.zip"
    $sourceArchive = Join-Path $distribution $sourceArchiveName
    & (Join-Path $PSScriptRoot 'Build-Source.ps1') -OutputPath $sourceArchive -Offline:$Offline
    $notices = Join-Path $distribution 'THIRD_PARTY_NOTICES.txt' 
    & (Join-Path $PSScriptRoot 'Generate-Notices.ps1') -OutputPath $notices
    $archiveItems = @('lasercat.exe','README.md','LICENSE','calibration','docs','LICENSES','THIRD_PARTY_NOTICES.txt') + $rootDocuments + @($sourceArchiveName)
    $archivePaths = $archiveItems | ForEach-Object { Join-Path $distribution $_ }
    Compress-Archive -LiteralPath $archivePaths -DestinationPath (Join-Path $distribution "LaserCat-$packageVersion-windows-x64.zip") -Force
    $compiler = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
    if (-not (Test-Path -LiteralPath $compiler)) { throw 'The Windows .NET Framework compiler is required for the installer; the portable package is ready.' }
    $installer = Join-Path $distribution "LaserCat-Setup-$packageVersion.exe"
    $source = Join-Path $projectRoot 'packaging\Installer.cs'
    $documentationResources = @()
    foreach ($document in $rootDocuments) { $documentationResources += "/resource:$projectRoot\$document,document:$document" }
    foreach ($document in (Get-ChildItem -LiteralPath (Join-Path $projectRoot 'docs'), (Join-Path $projectRoot 'LICENSES') -File -Recurse)) {
        $relativeDocument = $document.FullName.Substring($projectRoot.Length + 1).Replace('\','/')
        $documentationResources += "/resource:$($document.FullName),document:$relativeDocument"
    }
    & $compiler /nologo /target:winexe /optimize+ /platform:x64 /reference:System.Windows.Forms.dll /reference:System.Drawing.dll "/out:$installer" "/win32icon:$projectRoot\assets\icon.ico" "/resource:$projectRoot\assets\icon.ico,icon" "/resource:$binary,app" "/resource:$projectRoot\README.md,readme" "/resource:$projectRoot\LICENSE,license" "/resource:$notices,notices" "/resource:$projectRoot\calibration\engraving-calibration.lcc,engraving" "/resource:$projectRoot\calibration\cutting-calibration.lcc,cutting" "/resource:$sourceArchive,source" @documentationResources $source
    if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed.' }
    Get-ChildItem -LiteralPath $distribution -File | Select-Object Name, Length
} finally { Pop-Location }
