# Gnomcode Windows updater: use the same native installer as the download page.
$ErrorActionPreference = "Stop"
$Version = "1.2.1"
$BaseUrl = "https://gnomcode.vercel.app/downloads"
if (-not [Environment]::Is64BitOperatingSystem) {
    throw "gnomcode requires 64-bit Windows. The existing installation was not changed."
}
$Asset = "Gnomcode-v1.2.1-windows-x64.zip"
$Stage = Join-Path ([IO.Path]::GetTempPath()) ("gnomcode-update-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $Stage | Out-Null
try {
    $Archive = Join-Path $Stage $Asset
    Invoke-WebRequest -Uri "$BaseUrl/$Asset" -OutFile $Archive -UseBasicParsing
    $Manifest = (Invoke-WebRequest -Uri "$BaseUrl/SHA256SUMS-v$Version.txt" -UseBasicParsing).Content
    $Entry = @($Manifest -split "`n" | Where-Object { $_.Trim() -match ("^[a-fA-F0-9]{64}\s+" + [regex]::Escape($Asset) + "$") })
    if ($Entry.Count -ne 1) { throw "Missing or ambiguous release checksum." }
    $Expected = ($Entry[0].Trim() -split '\s+')[0]
    $Actual = (Get-FileHash -Path $Archive -Algorithm SHA256).Hash
    if ($Actual -ine $Expected) { throw "Release checksum verification failed. Existing install preserved." }
    Expand-Archive -Path $Archive -DestinationPath $Stage -Force
    $Setup = Join-Path $Stage "Gnomcode-Setup.exe"
    if (-not (Test-Path $Setup)) { throw "Release archive has no native installer." }
    $SetupVersion = [Diagnostics.FileVersionInfo]::GetVersionInfo($Setup).FileVersion
    if ($SetupVersion -ne "$Version.0") { throw "Installer version does not match the requested release." }
    Write-Output "Starting verified gnomcode v$Version installer. Your account and recorded sessions are preserved."
    $Process = Start-Process -FilePath $Setup -Wait -PassThru
    if ($Process.ExitCode -ne 0) { throw "gnomcode installer exited with code $($Process.ExitCode)." }
} finally {
    Remove-Item -Path $Stage -Recurse -Force -ErrorAction SilentlyContinue
}
