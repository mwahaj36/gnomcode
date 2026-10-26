# FleetHUD Windows Installer — https://fleethud.pro
$ErrorActionPreference = "Stop"

$InstallDir = "$env:LOCALAPPDATA\FleetHUD"
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$ExeDest = "$InstallDir\fleethud.exe"
$SourceExe = "$PSScriptRoot\..\target\release\fleethud.exe"

if (Test-Path $SourceExe) {
    Copy-Item -Path $SourceExe -Destination $ExeDest -Force
    Write-Output "Installed binary to $ExeDest"
} else {
    Write-Output "Fetching release binary..."
    $Repo = "mwahaj36/fleethud"
    $Arch = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }
    $ZipDest = "$env:TEMP\fleethud.zip"
    $Url = "https://github.com/$Repo/releases/latest/download/FleetHUD-v1.1.0-windows-$Arch.zip"
    Invoke-WebRequest -Uri $Url -OutFile $ZipDest
    Expand-Archive -Path $ZipDest -DestinationPath $InstallDir -Force
    Remove-Item $ZipDest -Force
}

# Copy launcher scripts
foreach ($script in @("tray.ps1", "run-tray.vbs", "open-dashboard.vbs")) {
    $src = "$PSScriptRoot\$script"
    if (Test-Path $src) {
        Copy-Item -Path $src -Destination "$InstallDir\$script" -Force
    }
}

# 1. Add to User PATH
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Output "Added $InstallDir to user PATH."
}

# 2. Create Desktop Shortcut (Zero Terminal Window)
$WScriptShell = New-Object -ComObject WScript.Shell
$DesktopShortcut = $WScriptShell.CreateShortcut("$env:USERPROFILE\Desktop\FleetHUD.lnk")
$DesktopShortcut.TargetPath = "wscript.exe"
$DesktopShortcut.Arguments = "`"$InstallDir\open-dashboard.vbs`""
$DesktopShortcut.IconLocation = "$InstallDir\fleethud.exe,0"
$DesktopShortcut.Description = "FleetHUD Mission Control for AI Coding Agents"
$DesktopShortcut.WorkingDirectory = $InstallDir
$DesktopShortcut.Save()
Write-Output "Created Desktop Shortcut: $env:USERPROFILE\Desktop\FleetHUD.lnk"

# 3. Create Start Menu Shortcut (Zero Terminal Window)
$StartMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\FleetHUD"
if (!(Test-Path $StartMenuDir)) {
    New-Item -ItemType Directory -Path $StartMenuDir -Force | Out-Null
}
$StartShortcut = $WScriptShell.CreateShortcut("$StartMenuDir\FleetHUD.lnk")
$StartShortcut.TargetPath = "wscript.exe"
$StartShortcut.Arguments = "`"$InstallDir\open-dashboard.vbs`""
$StartShortcut.IconLocation = "$InstallDir\fleethud.exe,0"
$StartShortcut.Description = "FleetHUD Mission Control for AI Coding Agents"
$StartShortcut.WorkingDirectory = $InstallDir
$StartShortcut.Save()
Write-Output "Created Start Menu Shortcut."

# 4. Configure Windows Startup on Boot (Silently launches daemon + system tray)
$StartupCmd = "wscript.exe `"$InstallDir\run-tray.vbs`""
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "FleetHUD" -Value $StartupCmd -Force
Write-Output "Registered Windows Startup autostart."

# 5. Create Uninstaller
$UninstallScript = @"
@echo off
taskkill /F /IM fleethud.exe 2>nul
powershell -Command "Get-Process -Name powershell | Where-Object { `$_.MainWindowTitle -eq '' } | Stop-Process" 2>nul
reg delete "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v FleetHUD /f 2>nul
del "%USERPROFILE%\Desktop\FleetHUD.lnk" 2>nul
rd /s /q "%APPDATA%\Microsoft\Windows\Start Menu\Programs\FleetHUD" 2>nul
echo FleetHUD has been uninstalled.
pause
"@
[System.IO.File]::WriteAllText("$InstallDir\uninstall.bat", $UninstallScript)

Write-Output "FleetHUD successfully installed!"

# Start Tray and Daemon now (Zero terminal window)
Start-Process "wscript.exe" -ArgumentList "`"$InstallDir\run-tray.vbs`""
# Open Mission Control in browser
Start-Process "wscript.exe" -ArgumentList "`"$InstallDir\open-dashboard.vbs`""
