param(
    [string]$ConfigPath = (Join-Path $PSScriptRoot "../config/repos.json"),
    [string]$WorkspaceRoot = "",
    [string]$Branch = "",
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

$repoRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$parentRoot = Split-Path $repoRoot -Parent

if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    $WorkspaceRoot = Join-Path $parentRoot $config.workspaceFolderName
}

New-Item -ItemType Directory -Path $WorkspaceRoot -Force | Out-Null

foreach ($repo in $config.repos) {
    $repoName = $repo.name
    $repoPath = Join-Path $WorkspaceRoot $repoName
    $cloneUrl = if ($repo.cloneUrl) { $repo.cloneUrl } else { Get-CloneUrl -GitHost $config.defaultHost -Owner $config.defaultOwner -RepoName $repoName }

    if (-not (Test-Path $repoPath)) {
        Write-Host "Cloning $repoName into $repoPath"
        & $gitExe clone $cloneUrl $repoPath
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to clone $repoName"
        }
    } else {
        Write-Host "Fetching updates for $repoName"
        Push-Location $repoPath
        try {
            & $gitExe fetch --all --prune
            if ($LASTEXITCODE -ne 0) {
                throw "Failed to fetch $repoName"
            }
        }
        finally {
            Pop-Location
        }
    }

    if (-not [string]::IsNullOrWhiteSpace($Branch)) {
        Push-Location $repoPath
        try {
            & $gitExe checkout $Branch
            if ($LASTEXITCODE -ne 0) {
                throw "Failed to checkout $Branch in $repoName"
            }
            & $gitExe pull --ff-only
            if ($LASTEXITCODE -ne 0) {
                throw "Failed to fast-forward $repoName on $Branch"
            }
        }
        finally {
            Pop-Location
        }
    }
}

Write-Host "Repository sync complete: $WorkspaceRoot"
