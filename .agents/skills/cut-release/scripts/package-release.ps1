# FleetHUD Automated Release Packager
param(
    [string]$Version
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RepoRoot = Resolve-Path "$ScriptDir\..\..\..\..\"

Set-Location $RepoRoot

# Detect version if not provided
if (-not $Version) {
    $setupCs = Get-Content "$RepoRoot\scripts\Setup.cs" -Raw
    if ($setupCs -match 'AssemblyVersion\("(\d+\.\d+\.\d+)\.?\d*"\)') {
        $Version = $Matches[1]
    } else {
        $Version = "1.1.2"
    }
}

Write-Host "=================================================" -ForegroundColor Cyan
Write-Host "  FleetHUD Release Packager - v$Version" -ForegroundColor Cyan
Write-Host "=================================================" -ForegroundColor Cyan

# 1. Build native Windows fleethud.exe
Write-Host "`n[1/4] Compiling native release fleethud.exe..." -ForegroundColor Yellow
$mingw = "C:\Users\Wahaj\AppData\Local\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin"
if (Test-Path $mingw) {
    $env:PATH = $mingw + ";" + $env:PATH
}
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
& $cargo build -p fleethud --release
if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }

Copy-Item "target\release\fleethud.exe" "web\public\downloads\fleethud.exe" -Force
Write-Host "  -> fleethud.exe copied to web\public\downloads" -ForegroundColor Green

# 2. Compile standalone Windows installer & uninstaller (FleetHUD-Setup.exe)
Write-Host "`n[2/4] Compiling FleetHUD-Setup.exe with embedded payload and uninstaller..." -ForegroundColor Yellow
$csc = "C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe"
$cscArgs = @(
    "/target:winexe",
    "/win32icon:scripts\fleethud.ico",
    "/resource:target\release\fleethud.exe,fleethud.exe",
    "/resource:scripts\tray.ps1,tray.ps1",
    "/resource:scripts\fleethud.ico,fleethud.ico",
    "/resource:scripts\run-tray.vbs,run-tray.vbs",
    "/resource:scripts\open-console.vbs,open-console.vbs",
    "/out:web\public\downloads\FleetHUD-Setup.exe",
    "scripts\Setup.cs"
)
& $csc $cscArgs
if ($LASTEXITCODE -ne 0) { throw "csc.exe compilation failed with exit code $LASTEXITCODE" }
Write-Host "  -> FleetHUD-Setup.exe compiled cleanly" -ForegroundColor Green

# 3. Create release zip archive
Write-Host "`n[3/4] Packaging FleetHUD-v$Version-windows-x64.zip..." -ForegroundColor Yellow
$zipPath = "web\public\downloads\FleetHUD-v$Version-windows-x64.zip"
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
$zipFiles = @(
    "target\release\fleethud.exe",
    "web\public\downloads\FleetHUD-Setup.exe",
    "scripts\fleethud.ico",
    "scripts\tray.ps1",
    "scripts\run-tray.vbs",
    "scripts\open-console.vbs"
)
Compress-Archive -Path $zipFiles -DestinationPath $zipPath
Write-Host "  -> $zipPath created successfully" -ForegroundColor Green

# 4. Build web production bundle
Write-Host "`n[4/4] Building Next.js web application..." -ForegroundColor Yellow
npm run build --prefix web
if ($LASTEXITCODE -ne 0) { throw "npm run build failed with exit code $LASTEXITCODE" }
Write-Host "  -> Web production build verified" -ForegroundColor Green

Write-Host "`n=================================================" -ForegroundColor Cyan
Write-Host "  Release Packaging Complete for v$Version!" -ForegroundColor Green
Write-Host "=================================================" -ForegroundColor Cyan
