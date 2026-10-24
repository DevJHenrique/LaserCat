# SPDX-License-Identifier: AGPL-3.0-only
param(
    [ValidateSet('debug','release')][string]$Profile = 'debug',
    [switch]$Offline
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot

function Invoke-CargoCheck {
    param([string[]]$CargoArguments)
    # Cargo reports progress on stderr. Windows PowerShell 5.1 turns redirected native stderr
    # into error records, which 'Stop' would make fatal; the exit code decides success instead.
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & cargo @CargoArguments
    } finally {
        $ErrorActionPreference = $previousPreference
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Verification failed (exit $LASTEXITCODE): cargo $($CargoArguments -join ' ')"
    }
}

$transcriptStarted = $false
Push-Location -LiteralPath $projectRoot
try {
    $logDirectory = Join-Path $projectRoot 'target/test-results'
    New-Item -ItemType Directory -Path $logDirectory -Force | Out-Null
    Start-Transcript -LiteralPath (Join-Path $logDirectory "$Profile.log") -Force | Out-Null
    $transcriptStarted = $true
    $dependencyArguments = @('--locked')
    if ($Offline) { $dependencyArguments += '--offline' }
    $profileArguments = @()
    if ($Profile -eq 'release') { $profileArguments += '--release' }
    Invoke-CargoCheck -CargoArguments @('fmt','--check')
    Invoke-CargoCheck -CargoArguments (@('clippy') + $dependencyArguments + @('--all-targets','--','-D','warnings'))
    Invoke-CargoCheck -CargoArguments (@('test') + $dependencyArguments + $profileArguments + @('--all-targets'))
    Invoke-CargoCheck -CargoArguments (@('test') + $dependencyArguments + $profileArguments + @('--doc'))
    Write-Host "Verification complete: $Profile. The script did not connect physical hardware."
} finally {
    if ($transcriptStarted) { Stop-Transcript | Out-Null }
    Pop-Location
}
