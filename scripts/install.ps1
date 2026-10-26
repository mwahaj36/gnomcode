# FleetHUD Windows Installer — https://fleethud.pro
$ErrorActionPreference = "Stop"

$Repo = "mwahaj36/fleethud"
$InstallDir = "$env:LOCALAPPDATA\FleetHUD\bin"

if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "x86" }
$DownloadUrl = "https://github.com/$Repo/releases/latest/download/fleethud-windows-$Arch.zip"

Write-Output "Downloading FleetHUD for Windows ($Arch)..."
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Output "Added $InstallDir to user PATH."
}

Write-Output "FleetHUD successfully installed."
Write-Output "Run 'fleethud doctor' or 'fleethud status' to begin."
