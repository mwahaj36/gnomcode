# spawnt Windows Installer — https://spawnt.xyz
$ErrorActionPreference = "Stop"

$InstallDir = "$env:LOCALAPPDATA\spawnt"
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$ExeDest = "$InstallDir\spawnt.exe"
$CandidateSources = @(
    "$PSScriptRoot\..\spawnt.exe",
    "$PSScriptRoot\spawnt.exe",
    "$PSScriptRoot\..\fleethud.exe",
    "$PSScriptRoot\fleethud.exe",
    "$PSScriptRoot\..\target\release\fleethud.exe",
    "$PSScriptRoot\..\target\release\spawnt.exe"
)
$FoundExe = $CandidateSources | Where-Object { Test-Path $_ } | Select-Object -First 1

if ($FoundExe) {
    Copy-Item -Path $FoundExe -Destination $ExeDest -Force
    # Backwards-compatible binary alias
    Copy-Item -Path $FoundExe -Destination "$InstallDir\fleethud.exe" -Force
    Write-Output "Installed binary to $ExeDest"
} else {
    Write-Output "Fetching release binary from https://spawnt.xyz..."
    $Arch = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }
    $ZipDest = "$env:TEMP\spawnt.zip"
    $Url = "https://fleethud.vercel.app/downloads/FleetHUD-v1.2.1-windows-$Arch.zip"
    Invoke-WebRequest -Uri $Url -OutFile $ZipDest
    Expand-Archive -Path $ZipDest -DestinationPath $InstallDir -Force
    if (Test-Path "$InstallDir\fleethud.exe") {
        Copy-Item "$InstallDir\fleethud.exe" -Destination $ExeDest -Force
    }
    Remove-Item $ZipDest -Force
}

# Copy launcher scripts & icon
foreach ($script in @("tray.ps1", "run-tray.vbs", "open-console.vbs", "open-dashboard.vbs", "spawnt.ico", "fleethud.ico")) {
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
$DesktopShortcut = $WScriptShell.CreateShortcut("$env:USERPROFILE\Desktop\spawnt.lnk")
$DesktopShortcut.TargetPath = "wscript.exe"
$DesktopShortcut.Arguments = "`"$InstallDir\open-console.vbs`""
$icoPath = if (Test-Path "$InstallDir\spawnt.ico") { "$InstallDir\spawnt.ico,0" } else { "$ExeDest,0" }
$DesktopShortcut.IconLocation = $icoPath
$DesktopShortcut.Description = "spawnt"
$DesktopShortcut.WorkingDirectory = $InstallDir
$DesktopShortcut.Save()
Write-Output "Created Desktop Shortcut: $env:USERPROFILE\Desktop\spawnt.lnk"

# 3. Create Start Menu Shortcut
$StartMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
$StartShortcut = $WScriptShell.CreateShortcut("$StartMenuDir\spawnt.lnk")
$StartShortcut.TargetPath = "wscript.exe"
$StartShortcut.Arguments = "`"$InstallDir\open-console.vbs`""
$StartShortcut.IconLocation = $icoPath
$StartShortcut.Description = "spawnt"
$StartShortcut.WorkingDirectory = $InstallDir
$StartShortcut.Save()

# 4. Configure Windows Startup Registry
$RunKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
Set-ItemProperty -Path $RunKey -Name "spawnt" -Value "wscript.exe `"$InstallDir\run-tray.vbs`"" -Force
Write-Output "Configured autostart in Windows Registry ($RunKey)."

# 5. Launch Tray & Console
Start-Process "wscript.exe" -ArgumentList "`"$InstallDir\run-tray.vbs`""
Start-Process "wscript.exe" -ArgumentList "`"$InstallDir\open-console.vbs`""

Write-Output "spawnt v1.2.1 installation complete. Running in system tray."
