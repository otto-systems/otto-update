param(
    [string]$ConfigPath = (Join-Path $PSScriptRoot "../config/repos.json"),
    [string]$GitPath = ""
)

$ErrorActionPreference = "Stop"

function Get-CloneUrl {
    param(
        [string]$GitHost,
        [string]$Owner,
        [string]$RepoName
    )

    return "https://$GitHost/$Owner/$RepoName.git"
}

function Resolve-GitPath {
    param([string]$Candidate)

    if (-not [string]::IsNullOrWhiteSpace($Candidate)) {
        if (-not (Test-Path $Candidate)) {
            throw "Provided GitPath was not found: $Candidate"
        }
        return $Candidate
    }

    $gitCommand = Get-Command git -ErrorAction SilentlyContinue
    if ($gitCommand) {
        return $gitCommand.Source
    }

    $commonPaths = @(
        "C:/Program Files/Git/cmd/git.exe",
        "C:/Program Files/Git/bin/git.exe",
        "C:/Program Files (x86)/Git/cmd/git.exe"
    )

    foreach ($path in $commonPaths) {
        if (Test-Path $path) {
            return $path
        }
    }

    throw "git was not found. Install git or pass -GitPath to this script."
}

$gitExe = Resolve-GitPath -Candidate $GitPath

if (-not (Test-Path $ConfigPath)) {
    throw "Config not found: $ConfigPath"
}

$config = Get-Content -Path $ConfigPath -Raw | ConvertFrom-Json

$ghAvailable = [bool](Get-Command gh -ErrorAction SilentlyContinue)
if ($ghAvailable) {
    gh auth status | Out-Null
}

$results = @()
$requiredFailures = 0

foreach ($repo in $config.repos) {
    $repoName = $repo.name
    $required = [bool]$repo.required
    $cloneUrl = if ($repo.cloneUrl) { $repo.cloneUrl } else { Get-CloneUrl -GitHost $config.defaultHost -Owner $config.defaultOwner -RepoName $repoName }

    & $gitExe ls-remote $cloneUrl HEAD *> $null
    $ok = ($LASTEXITCODE -eq 0)

    if (-not $ok -and $required) {
        $requiredFailures += 1
    }

    $results += [pscustomobject]@{
        Repo = $repoName
        Required = $required
        CloneUrl = $cloneUrl
        AccessOk = $ok
    }
}

$results | Sort-Object Repo | Format-Table -AutoSize

if ($requiredFailures -gt 0) {
    throw "Preflight failed for $requiredFailures required repositories."
}

Write-Host "Preflight passed for all required repositories."
