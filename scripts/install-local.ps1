$ErrorActionPreference = 'Stop'
$InstallDir = Join-Path $env:LOCALAPPDATA 'Gnomcode'
Write-Host "Installing fresh Gnomcode to $InstallDir..."

if (Test-Path $InstallDir) {
    Remove-Item -Path $InstallDir -Recurse -Force
}
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

Copy-Item "target/release/gnomcode.exe" (Join-Path $InstallDir "gnomcode.exe") -Force
Copy-Item "web/public/downloads/Gnomcode-Setup.exe" (Join-Path $InstallDir "Gnomcode-Uninstall.exe") -Force
Copy-Item "LICENSE" (Join-Path $InstallDir "LICENSE") -Force
Copy-Item "web/public/downloads/THIRD_PARTY_NOTICES.html" (Join-Path $InstallDir "THIRD_PARTY_NOTICES.html") -Force

foreach ($file in @('tray.ps1', 'gnomcode.ico', 'gnomcode-tray.ico', 'run-tray.vbs', 'open-console.vbs', 'open-dashboard.vbs', 'open-folder.ps1')) {
    Copy-Item (Join-Path "scripts" $file) (Join-Path $InstallDir $file) -Force
}

$DesktopDest = Join-Path $InstallDir "desktop"
New-Item -ItemType Directory -Path $DesktopDest -Force | Out-Null
Copy-Item "apps/desktop/out/Gnomcode-win32-x64/*" $DesktopDest -Recurse -Force

# Register AppUserModelId
$AppModelKey = "HKCU:\Software\Classes\AppUserModelId\app.gnomcode.desktop"
if (-not (Test-Path $AppModelKey)) { New-Item -Path $AppModelKey -Force | Out-Null }
Set-ItemProperty -Path $AppModelKey -Name "DisplayName" -Value "Gnomcode"
Set-ItemProperty -Path $AppModelKey -Name "IconUri" -Value (Join-Path $InstallDir "gnomcode.ico")

# Register Uninstall entry
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Gnomcode"
if (-not (Test-Path $UninstallKey)) { New-Item -Path $UninstallKey -Force | Out-Null }
Set-ItemProperty -Path $UninstallKey -Name "DisplayName" -Value "gnomcode"
Set-ItemProperty -Path $UninstallKey -Name "DisplayVersion" -Value "0.9.3"
Set-ItemProperty -Path $UninstallKey -Name "Publisher" -Value "gnomcode"
Set-ItemProperty -Path $UninstallKey -Name "DisplayIcon" -Value ((Join-Path $InstallDir "gnomcode.ico") + ",0")
Set-ItemProperty -Path $UninstallKey -Name "InstallLocation" -Value $InstallDir
Set-ItemProperty -Path $UninstallKey -Name "UninstallString" -Value ('"' + (Join-Path $InstallDir "Gnomcode-Uninstall.exe") + '" --uninstall')
Set-ItemProperty -Path $UninstallKey -Name "QuietUninstallString" -Value ('"' + (Join-Path $InstallDir "Gnomcode-Uninstall.exe") + '" --uninstall --quiet')
Set-ItemProperty -Path $UninstallKey -Name "EstimatedSize" -Value 16500 -Type DWord
Set-ItemProperty -Path $UninstallKey -Name "NoModify" -Value 1 -Type DWord
Set-ItemProperty -Path $UninstallKey -Name "NoRepair" -Value 1 -Type DWord
Set-ItemProperty -Path $UninstallKey -Name "URLInfoAbout" -Value "https://gnomcode.vercel.app"
Set-ItemProperty -Path $UninstallKey -Name "HelpLink" -Value "https://gnomcode.vercel.app/docs"

# Create Shortcuts
$WshShell = New-Object -ComObject WScript.Shell
$Ico = Join-Path $InstallDir "gnomcode.ico"
$DesktopExe = Join-Path $InstallDir "desktop\gnomcode-desktop.exe"

$DesktopDir = [Environment]::GetFolderPath('DesktopDirectory')
$DesktopLnk = Join-Path $DesktopDir "gnomcode.lnk"
$Shortcut = $WshShell.CreateShortcut($DesktopLnk)
$Shortcut.TargetPath = $DesktopExe
$Shortcut.IconLocation = "$Ico,0"
$Shortcut.Description = "gnomcode"
$Shortcut.WorkingDirectory = $InstallDir
$Shortcut.Save()

$StartMenuDir = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'Microsoft\Windows\Start Menu\Programs'
$StartLnk = Join-Path $StartMenuDir "gnomcode.lnk"
$Shortcut = $WshShell.CreateShortcut($StartLnk)
$Shortcut.TargetPath = $DesktopExe
$Shortcut.IconLocation = "$Ico,0"
$Shortcut.Description = "gnomcode"
$Shortcut.WorkingDirectory = $InstallDir
$Shortcut.Save()

# User PATH
$CurPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$CurPath;$InstallDir", "User")
}

Write-Host "Gnomcode v0.9.3 installed successfully to $InstallDir!"
