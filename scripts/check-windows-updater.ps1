# Run with PowerShell in a disposable environment. Downloads and GUI launch are mocked;
# checksum verification, archive extraction and PE file-version checking are real.
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Source = Get-Content "$Root/scripts/install.ps1" -Raw
if ($Source -notmatch '\$Version = "([^"]+)"') { throw "Updater version missing" }
$Version = $Matches[1]
$global:FleetHudCheckArchivePath = "$Root/web/public/downloads/FleetHUD-v$Version-windows-x64.zip"
$global:FleetHudCheckManifestPath = "$Root/web/public/downloads/SHA256SUMS-v$Version.txt"
$global:FleetHudCheckLaunchCount = 0
$global:FleetHudCheckCorruptArchive = $false
function Invoke-WebRequest {
    param([string]$Uri, [string]$OutFile, [switch]$UseBasicParsing)
    if ($OutFile) {
        Copy-Item $global:FleetHudCheckArchivePath $OutFile
        if ($global:FleetHudCheckCorruptArchive) {
            $Bytes = [IO.File]::ReadAllBytes($OutFile)
            $Bytes[0] = $Bytes[0] -bxor 1
            [IO.File]::WriteAllBytes($OutFile, $Bytes)
        }
    } else {
        return [pscustomobject]@{ Content = (Get-Content $global:FleetHudCheckManifestPath -Raw) }
    }
}
function Start-Process {
    param([string]$FilePath, [switch]$Wait, [switch]$PassThru)
    if (-not (Test-Path $FilePath)) { throw "Installer was not extracted" }
    $global:FleetHudCheckLaunchCount++
    return [pscustomobject]@{ ExitCode = 0 }
}
$Errors = $null
$Tokens = $null
foreach ($Path in @("$Root/scripts/install.ps1", "$Root/.agents/skills/cut-release/scripts/package-release.ps1")) {
    [Management.Automation.Language.Parser]::ParseFile($Path, [ref]$Tokens, [ref]$Errors) | Out-Null
    if ($Errors.Count) { throw ($Errors | Out-String) }
}
& "$Root/scripts/install.ps1"
if ($global:FleetHudCheckLaunchCount -ne 1) { throw "Verified installer was not launched once" }
$global:FleetHudCheckCorruptArchive = $true
$global:FleetHudCheckLaunchCount = 0
$Rejected = $false
try { & "$Root/scripts/install.ps1" } catch {
    if ($_.Exception.Message -notlike '*checksum verification failed*') { throw }
    $Rejected = $true
}
if (-not $Rejected -or $global:FleetHudCheckLaunchCount -ne 0) { throw "Corrupt update was not rejected before execution" }
Write-Output "PASS: PowerShell parsing, valid archive, PE version, launch wiring, corrupt archive rejection"
