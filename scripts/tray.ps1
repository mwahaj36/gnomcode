# FleetHUD Windows System Tray Service — https://fleethud.vercel.app
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$Port = 41738
$DaemonInfoPath = "$env:USERPROFILE\.fleethud\daemon.json"

function Get-ConsoleUrl {
    if (Test-Path $DaemonInfoPath) {
        try {
            $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
            return "http://127.0.0.1:$($info.port)/?token=$($info.token)"
        } catch {}
    }
    return "http://127.0.0.1:$Port/"
}

function Open-Console {
    $url = Get-ConsoleUrl
    try {
        Start-Process "msedge.exe" -ArgumentList "--app=$url --window-size=1280,850" -ErrorAction Stop
    } catch {
        try {
            Start-Process "chrome.exe" -ArgumentList "--app=$url --window-size=1280,850" -ErrorAction Stop
        } catch {
            Start-Process $url
        }
    }
}

$notifyIcon = New-Object System.Windows.Forms.NotifyIcon
$notifyIcon.Icon = [System.Drawing.SystemIcons]::Application
$notifyIcon.Text = "FleetHUD Mission Control Console"
$notifyIcon.Visible = $true

$contextMenu = New-Object System.Windows.Forms.ContextMenuStrip

$openItem = $contextMenu.Items.Add("Open Console")
$openItem.Font = New-Object System.Drawing.Font($openItem.Font, [System.Drawing.FontStyle]::Bold)
$openItem.Add_Click({ Open-Console })

$sessionsItem = $contextMenu.Items.Add("Active Sessions: 0")
$sessionsItem.Enabled = $false

$contextMenu.Items.Add("-") | Out-Null

$exitItem = $contextMenu.Items.Add("Exit")
$exitItem.Add_Click({
    $notifyIcon.Visible = $false
    [System.Windows.Forms.Application]::Exit()
    Exit
})

$notifyIcon.ContextMenuStrip = $contextMenu
$notifyIcon.Add_DoubleClick({ Open-Console })
$notifyIcon.Add_Click({
    param($sender, $e)
    if ($e.Button -eq [System.Windows.Forms.MouseButtons]::Left) {
        Open-Console
    }
})

# Timer to update active sessions count every 3 seconds
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 3000
$timer.Add_Tick({
    try {
        if (Test-Path $DaemonInfoPath) {
            $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
            $res = Invoke-RestMethod -Uri "http://127.0.0.1:$($info.port)/api/v1/live?page=1&limit=1" -TimeoutSec 1 -ErrorAction SilentlyContinue
            if ($res) {
                $count = $res.active_count
                $sessionsItem.Text = "Active Sessions: $count"
                if ($count -gt 0) {
                    $notifyIcon.Text = "FleetHUD — $count Active Agents Running"
                } else {
                    $notifyIcon.Text = "FleetHUD — Fleet Idle"
                }
            }
        }
    } catch {}
})
$timer.Start()

[System.Windows.Forms.Application]::Run()
