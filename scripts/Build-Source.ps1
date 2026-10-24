# SPDX-License-Identifier: AGPL-3.0-only
param([Parameter(Mandatory=$true)][string]$OutputPath, [switch]$Offline)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $projectRoot
try {
    $stage = Join-Path $projectRoot ("target\source-package\" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    # An explicit allowlist excludes local profiles, credentials, Git data and build outputs.
    $items = @('src','tests','examples','scripts','packaging','calibration','docs','LICENSES','.github','assets','locales',
        'Cargo.toml','Cargo.lock','rust-toolchain.toml','.gitignore','LICENSE','NOTICE','README.md',
        'ARCHITECTURE.md','CREDITS.md','CONTRIBUTING.md','CHANGELOG.md','SECURITY.md','AGENTS.md','CLAUDE.md','agent.md',
        'README.pt-BR.md','CODE_OF_CONDUCT.md','.gitattributes','.editorconfig')
    foreach ($item in $items) { Copy-Item -LiteralPath (Join-Path $projectRoot $item) -Destination $stage -Recurse -Force }
    # Shared agent configuration only; personal settings.local.json stays out of the archive.
    $agentStage = Join-Path $stage '.claude'
    New-Item -ItemType Directory -Path $agentStage -Force | Out-Null
    foreach ($item in @('settings.json','agents','skills')) {
        Copy-Item -LiteralPath (Join-Path $projectRoot ".claude\$item") -Destination $agentStage -Recurse -Force
    }
    $vendor = Join-Path $stage 'vendor'
    $vendorArguments = @('vendor','--locked','--quiet')
    if ($Offline) { $vendorArguments += '--offline' }
    & cargo @vendorArguments $vendor | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Could not vendor the pinned dependency sources.' }
    $configDirectory = Join-Path $stage '.cargo'
    New-Item -ItemType Directory -Path $configDirectory -Force | Out-Null
    $config = @'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
'@
    [IO.File]::WriteAllText((Join-Path $configDirectory 'config.toml'), $config, [Text.UTF8Encoding]::new($false))
    $destination = [IO.Path]::GetFullPath($OutputPath)
    New-Item -ItemType Directory -Path ([IO.Path]::GetDirectoryName($destination)) -Force | Out-Null
    # ZipFile retains dotfiles, especially vendor checksums and .cargo/config.toml.
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    if (Test-Path -LiteralPath $destination) { Remove-Item -LiteralPath $destination -Force }
    [IO.Compression.ZipFile]::CreateFromDirectory($stage, $destination, [IO.Compression.CompressionLevel]::Optimal, $false)
    Write-Host "Corresponding source: $destination"
} finally { Pop-Location }
