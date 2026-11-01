param([Parameter(Mandatory = $true)][string]$Folder)
$ErrorActionPreference = 'Stop'

function Get-GnomcodeFolderUrl($Info, [string]$Path) {
    $port = [int]$Info.port
    if ($port -lt 1 -or $port -gt 65535 -or [string]::IsNullOrWhiteSpace($Info.token)) { throw 'Invalid local daemon connection.' }
    return 'http://127.0.0.1:' + $port + '/?token=' + [Uri]::EscapeDataString($Info.token).Replace("'", '%27') + '#sessions?folder=' + [Uri]::EscapeDataString($Path).Replace("'", '%27')
}

try {
    $workspace = (Get-Item -LiteralPath $Folder -ErrorAction Stop)
    if (-not $workspace.PSIsContainer) { throw 'Choose a folder to open with Gnomcode.' }
    $gnomcodeData = if ($env:GNOMCODE_HOME -and $env:GNOMCODE_HOME.Trim()) { $env:GNOMCODE_HOME.Trim() } elseif ($env:SPAWNT_HOME -and $env:SPAWNT_HOME.Trim()) { $env:SPAWNT_HOME.Trim() } elseif (Test-Path (Join-Path $env:USERPROFILE '.spawnt')) { Join-Path $env:USERPROFILE '.spawnt' } else { Join-Path $env:USERPROFILE '.gnomcode' }
    $infoPath = Join-Path $gnomcodeData 'daemon.json'
    Start-Process -FilePath (Join-Path $PSScriptRoot 'gnomcode.exe') -ArgumentList 'daemon' -WindowStyle Hidden
    $info = $null
    for ($attempt = 0; $attempt -lt 50; $attempt++) {
        try {
            $candidate = Get-Content -LiteralPath $infoPath -Raw | ConvertFrom-Json
            if (Get-Process -Id $candidate.pid -ErrorAction SilentlyContinue) { $info = $candidate; break }
        } catch {}
        Start-Sleep -Milliseconds 200
    }
    if (-not $info) { throw 'Gnomcode could not start. Open Gnomcode and try again.' }
    Start-Process (Get-GnomcodeFolderUrl $info $workspace.FullName)
} catch {
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.MessageBox]::Show($_.Exception.Message, 'Open with Gnomcode', 'OK', 'Error') | Out-Null
    exit 1
}
