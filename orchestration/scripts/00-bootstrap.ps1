param(
    [string]$ConfigPath = (Join-Path $PSScriptRoot "../config/repos.json"),
    [string]$MatrixPath = (Join-Path $PSScriptRoot "../config/matrix.json"),
    [string]$WorkspaceRoot = "",
    [string]$GitPath = "",
    [switch]$SkipMatrix
)

$ErrorActionPreference = "Stop"

$preflight = Join-Path $PSScriptRoot "01-preflight-access.ps1"
$sync = Join-Path $PSScriptRoot "02-sync-repos.ps1"
$matrix = Join-Path $PSScriptRoot "03-run-matrix.ps1"

& $preflight -ConfigPath $ConfigPath -GitPath $GitPath
& $sync -ConfigPath $ConfigPath -WorkspaceRoot $WorkspaceRoot -GitPath $GitPath

if (-not $SkipMatrix) {
    & $matrix -MatrixPath $MatrixPath -WorkspaceRoot $WorkspaceRoot
}

Write-Host "Bootstrap workflow complete."
