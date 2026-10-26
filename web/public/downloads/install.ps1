# FleetHUD Windows Installer — https://fleethud.vercel.app
$ErrorActionPreference = "Stop"

$InstallDir = "$env:LOCALAPPDATA\FleetHUD"
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$ExeDest = "$InstallDir\fleethud.exe"
$CandidateSources = @(
    "$PSScriptRoot\..\fleethud.exe",
    "$PSScriptRoot\fleethud.exe",
    "$PSScriptRoot\..\target\release\fleethud.exe"
)
$FoundExe = $CandidateSources | Where-Object { Test-Path $_ } | Select-Object -First 1

if ($FoundExe) {
    Copy-Item -Path $FoundExe -Destination $ExeDest -Force
    Write-Output "Installed binary to $ExeDest"
} else {
    Write-Output "Fetching release binary from https://fleethud.vercel.app..."
    $Arch = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }
    $ZipDest = "$env:TEMP\fleethud.zip"
    $Url = "https://fleethud.vercel.app/downloads/FleetHUD-v1.1.2-windows-$Arch.zip"
    Invoke-WebRequest -Uri $Url -OutFile $ZipDest
    Expand-Archive -Path $ZipDest -DestinationPath $InstallDir -Force
    Remove-Item $ZipDest -Force
}

# Copy launcher scripts
foreach ($script in @("tray.ps1", "run-tray.vbs", "open-console.vbs", "open-dashboard.vbs")) {
    $srcCandidates = @(
        "$PSScriptRoot\$script",
        "$PSScriptRoot\scripts\$script",
        "$PSScriptRoot\..\scripts\$script"
    )
    $foundScript = $srcCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if ($foundScript) {
        Copy-Item -Path $foundScript -Destination "$InstallDir\$script" -Force
    }
}

# 1. Add to User PATH
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Output "Added $InstallDir to user PATH."
}

# 2. Create Desktop Shortcut
$WScriptShell = New-Object -ComObject WScript.Shell
foreach ($lnkPath in @("$env:USERPROFILE\Desktop\FleetHUD Console.lnk", "$env:USERPROFILE\Desktop\FleetHUD.lnk")) {
    $DesktopShortcut = $WScriptShell.CreateShortcut($lnkPath)
    $DesktopShortcut.TargetPath = "wscript.exe"
    $DesktopShortcut.Arguments = "`"$InstallDir\open-console.vbs`""
    $DesktopShortcut.IconLocation = "$InstallDir\fleethud.exe,0"
    $DesktopShortcut.Description = "FleetHUD Mission Control Console"
    $DesktopShortcut.WorkingDirectory = $InstallDir
    $DesktopShortcut.Save()
}
Write-Output "Created Desktop Shortcut: $env:USERPROFILE\Desktop\FleetHUD Console.lnk"

# 3. Create Start Menu Shortcut
$StartMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
$StartShortcut = $WScriptShell.CreateShortcut("$StartMenuDir\FleetHUD Console.lnk")
$StartShortcut.TargetPath = "wscript.exe"
$StartShortcut.Arguments = "`"$InstallDir\open-console.vbs`""
$StartShortcut.IconLocation = "$InstallDir\fleethud.exe,0"
$StartShortcut.Description = "FleetHUD Mission Control Console"
$StartShortcut.WorkingDirectory = $InstallDir
$StartShortcut.Save()

# 4. Configure Windows Startup Registry
$RunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
Set-ItemProperty -Path $RunKey -Name "FleetHUD" -Value "wscript.exe `"$InstallDir\run-tray.vbs`"" -Force
Write-Output "Configured autostart in Windows Registry ($RunKey)."

# 5. Launch Tray & Console
Start-Process "wscript.exe" -ArgumentList "`"$InstallDir\run-tray.vbs`""
Start-Process "wscript.exe" -ArgumentList "`"$InstallDir\open-console.vbs`""

Write-Output "FleetHUD v1.1.2 installation complete. Mission Control running in system tray."
