# Run with PowerShell in a disposable environment. Downloads and GUI launch are mocked;
# checksum verification, archive extraction and PE file-version checking are real.
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Source = Get-Content "$Root/scripts/install.ps1" -Raw
if ($Source -notmatch '\$Version = "([^"]+)"') { throw "Updater version missing" }
$Version = $Matches[1]
$global:GnomcodeCheckArchivePath = "$Root/web/public/downloads/Gnomcode-v$Version-windows-x64.zip"
$global:GnomcodeCheckManifestPath = "$Root/web/public/downloads/SHA256SUMS-v$Version.txt"
$global:GnomcodeCheckLaunchCount = 0
$global:GnomcodeCheckCorruptArchive = $false
function Invoke-WebRequest {
    param([string]$Uri, [string]$OutFile, [switch]$UseBasicParsing)
    if ($OutFile) {
        Copy-Item $global:GnomcodeCheckArchivePath $OutFile
        if ($global:GnomcodeCheckCorruptArchive) {
            $Bytes = [IO.File]::ReadAllBytes($OutFile)
            $Bytes[0] = $Bytes[0] -bxor 1
            [IO.File]::WriteAllBytes($OutFile, $Bytes)
        }
    } else {
        return [pscustomobject]@{ Content = (Get-Content $global:GnomcodeCheckManifestPath -Raw) }
    }
}
function Start-Process {
    param([string]$FilePath, [switch]$Wait, [switch]$PassThru)
    if (-not (Test-Path $FilePath)) { throw "Installer was not extracted" }
    $global:GnomcodeCheckLaunchCount++
    return [pscustomobject]@{ ExitCode = 0 }
}
$Errors = $null
$Tokens = $null
foreach ($Path in @("$Root/scripts/install.ps1", "$Root/.agents/skills/cut-release/scripts/package-release.ps1")) {
    [Management.Automation.Language.Parser]::ParseFile($Path, [ref]$Tokens, [ref]$Errors) | Out-Null
    if ($Errors.Count) { throw ($Errors | Out-String) }
}
& "$Root/scripts/install.ps1"
if ($global:GnomcodeCheckLaunchCount -ne 1) { throw "Verified installer was not launched once" }
$global:GnomcodeCheckCorruptArchive = $true
$global:GnomcodeCheckLaunchCount = 0
$Rejected = $false
try { & "$Root/scripts/install.ps1" } catch {
    if ($_.Exception.Message -notlike '*checksum verification failed*') { throw }
    $Rejected = $true
}
if (-not $Rejected -or $global:GnomcodeCheckLaunchCount -ne 0) { throw "Corrupt update was not rejected before execution" }
Write-Output "PASS: PowerShell parsing, valid archive, PE version, launch wiring, corrupt archive rejection"
