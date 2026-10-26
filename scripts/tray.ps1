# FleetHUD Windows System Tray Service — https://fleethud.pro
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$Port = 41738
$DaemonInfoPath = "$env:USERPROFILE\.fleethud\daemon.json"

function Get-DashboardUrl {
    if (Test-Path $DaemonInfoPath) {
        try {
            $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
            return "http://127.0.0.1:$($info.port)/?token=$($info.token)"
        } catch {}
    }
    return "http://127.0.0.1:$Port/"
}

function Open-Dashboard {
    $url = Get-DashboardUrl
    Start-Process $url
}

$notifyIcon = New-Object System.Windows.Forms.NotifyIcon
$notifyIcon.Icon = [System.Drawing.SystemIcons]::Application
$notifyIcon.Text = "FleetHUD Mission Control"
$notifyIcon.Visible = $true

$contextMenu = New-Object System.Windows.Forms.ContextMenuStrip

$openItem = $contextMenu.Items.Add("Open Mission Control")
$openItem.Font = New-Object System.Drawing.Font($openItem.Font, [System.Drawing.FontStyle]::Bold)
$openItem.Add_Click({ Open-Dashboard })

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
$notifyIcon.Add_DoubleClick({ Open-Dashboard })
$notifyIcon.Add_Click({
    param($sender, $e)
    if ($e.Button -eq [System.Windows.Forms.MouseButtons]::Left) {
        Open-Dashboard
    }
})

# Timer to update active sessions count every 3 seconds
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 3000
$timer.Add_Tick({
    try {
        if (Test-Path $DaemonInfoPath) {
            $info = Get-Content $DaemonInfoPath -Raw -ErrorAction SilentlyContinue | ConvertFrom-Json
            if ($info -and $info.port -and $info.token) {
                $uri = "http://127.0.0.1:$($info.port)/api/v1/sessions?limit=50"
                $headers = @{ "Authorization" = "Bearer $($info.token)" }
                $res = Invoke-RestMethod -Uri $uri -Headers $headers -TimeoutSec 2 -ErrorAction SilentlyContinue
                if ($res -and $res.sessions) {
                    $active = ($res.sessions | Where-Object { $_.is_active -eq $true }).Count
                    $sessionsItem.Text = "Active Sessions: $active"
                    if ($active -gt 0) {
                        $notifyIcon.Text = "FleetHUD: [RUN] $active active session(s)"
                    } else {
                        $notifyIcon.Text = "FleetHUD: [IDLE] Ready"
                    }
                }
            }
        }
    } catch {}
})
$timer.Start()

[System.Windows.Forms.Application]::Run()
