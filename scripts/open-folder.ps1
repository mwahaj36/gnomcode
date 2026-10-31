param([Parameter(Mandatory = $true)][string]$Folder)
$ErrorActionPreference = 'Stop'

function Get-SpawntFolderUrl($Info, [string]$Path) {
    $port = [int]$Info.port
    if ($port -lt 1 -or $port -gt 65535 -or [string]::IsNullOrWhiteSpace($Info.token)) { throw 'Invalid local daemon connection.' }
    return 'http://127.0.0.1:' + $port + '/?token=' + [Uri]::EscapeDataString($Info.token).Replace("'", '%27') + '#sessions?folder=' + [Uri]::EscapeDataString($Path).Replace("'", '%27')
}

try {
    $workspace = (Get-Item -LiteralPath $Folder -ErrorAction Stop)
    if (-not $workspace.PSIsContainer) { throw 'Choose a folder to open with Spawnt.' }
    $spawntData = if ($env:SPAWNT_HOME) { $env:SPAWNT_HOME } else { Join-Path $env:USERPROFILE '.spawnt' }
    $infoPath = Join-Path $spawntData 'daemon.json'
    Start-Process -FilePath (Join-Path $PSScriptRoot 'spawnt.exe') -ArgumentList 'daemon' -WindowStyle Hidden
    $info = $null
    for ($attempt = 0; $attempt -lt 50; $attempt++) {
        try {
            $candidate = Get-Content -LiteralPath $infoPath -Raw | ConvertFrom-Json
            if (Get-Process -Id $candidate.pid -ErrorAction SilentlyContinue) { $info = $candidate; break }
        } catch {}
        Start-Sleep -Milliseconds 200
    }
    if (-not $info) { throw 'Spawnt could not start. Open Spawnt and try again.' }
    Start-Process (Get-SpawntFolderUrl $info $workspace.FullName)
} catch {
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.MessageBox]::Show($_.Exception.Message, 'Open with Spawnt', 'OK', 'Error') | Out-Null
    exit 1
}
