$ErrorActionPreference = 'Stop'

$ServiceName = 'OttoUpdate'
$BinaryPath = 'C:\Program Files\OttoUpdate\ottoupdate-server.exe'
$ConfigPath = 'C:\ProgramData\OttoUpdate\config.toml'

New-Item -ItemType Directory -Path 'C:\Program Files\OttoUpdate' -Force | Out-Null
New-Item -ItemType Directory -Path 'C:\ProgramData\OttoUpdate' -Force | Out-Null

Copy-Item -Path '.\target\release\ottoupdate-server.exe' -Destination $BinaryPath -Force

if (-not (Test-Path $ConfigPath)) {
@'
[server]
bind = "127.0.0.1:7430"
'@ | Set-Content -Path $ConfigPath -Encoding UTF8
}

sc.exe delete $ServiceName | Out-Null
sc.exe create $ServiceName binPath= "`"$BinaryPath`" --config `"$ConfigPath`"" start= auto
sc.exe start $ServiceName

Write-Host "$ServiceName installed and started"
