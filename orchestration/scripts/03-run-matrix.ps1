param(
    [string]$MatrixPath = (Join-Path $PSScriptRoot "../config/matrix.json"),
    [string]$WorkspaceRoot = "",
    [switch]$FailFast
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $MatrixPath)) {
    throw "Matrix config not found: $MatrixPath"
}

$repoRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$parentRoot = Split-Path $repoRoot -Parent

if ([string]::IsNullOrWhiteSpace($WorkspaceRoot)) {
    $WorkspaceRoot = Join-Path $parentRoot "otto-workspace"
}

$matrix = Get-Content -Path $MatrixPath -Raw | ConvertFrom-Json
$logsDir = Join-Path (Split-Path $PSScriptRoot -Parent) "logs"
New-Item -ItemType Directory -Path $logsDir -Force | Out-Null

$jobs = @()
$index = 0

foreach ($lane in $matrix.lanes) {
    $index += 1
    $jobName = "lane-$index-$($lane.repo)"

    $jobs += Start-Job -Name $jobName -ScriptBlock {
        param($laneSpec, $workspace, $outputDir)

        $ErrorActionPreference = "Stop"

        $repoPath = Join-Path $workspace $laneSpec.repo
        if ($laneSpec.workingDirectory) {
            $repoPath = Join-Path $repoPath $laneSpec.workingDirectory
        }

        $safeRepo = $laneSpec.repo -replace "[^a-zA-Z0-9_-]", "_"
        $timeStamp = Get-Date -Format "yyyyMMdd-HHmmss"
        $logPath = Join-Path $outputDir "$safeRepo-$timeStamp.log"

        if (-not (Test-Path $repoPath)) {
            return [pscustomobject]@{
                Repo = $laneSpec.repo
                Lane = $laneSpec.name
                Success = $false
                LogPath = $logPath
                FailedCommand = "(repo path missing)"
            }
        }

        New-Item -ItemType File -Path $logPath -Force | Out-Null

        Push-Location $repoPath
        try {
            foreach ($command in $laneSpec.commands) {
                Add-Content -Path $logPath -Value ">>> $command"
                Add-Content -Path $logPath -Value ""

                $output = & cmd.exe /d /s /c $command 2>&1
                $exitCode = $LASTEXITCODE

                if ($output) {
                    $output | Out-File -FilePath $logPath -Append
                }

                Add-Content -Path $logPath -Value ""
                Add-Content -Path $logPath -Value "ExitCode: $exitCode"
                Add-Content -Path $logPath -Value "----------------------------------------"

                if ($exitCode -ne 0) {
                    return [pscustomobject]@{
                        Repo = $laneSpec.repo
                        Lane = $laneSpec.name
                        Success = $false
                        LogPath = $logPath
                        FailedCommand = $command
                    }
                }
            }

            return [pscustomobject]@{
                Repo = $laneSpec.repo
                Lane = $laneSpec.name
                Success = $true
                LogPath = $logPath
                FailedCommand = ""
            }
        }
        finally {
            Pop-Location
        }
    } -ArgumentList $lane, $WorkspaceRoot, $logsDir
}

$results = @()

foreach ($job in $jobs) {
    $jobResult = Receive-Job -Job $job -Wait
    Remove-Job -Job $job

    $results += $jobResult

    if ($FailFast -and (-not $jobResult.Success)) {
        Write-Host "FailFast enabled, stopping early after failure in $($jobResult.Repo)."
        break
    }
}

$results | Sort-Object Repo, Lane | Format-Table Repo, Lane, Success, FailedCommand, LogPath -AutoSize

$failedCount = ($results | Where-Object { -not $_.Success }).Count
if ($failedCount -gt 0) {
    throw "Matrix run finished with $failedCount failed lane(s)."
}

Write-Host "Matrix run passed."
