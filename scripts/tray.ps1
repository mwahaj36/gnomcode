# FleetHUD Windows System Tray Service — https://fleethud.vercel.app
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$Port = 41738
$DaemonInfoPath = "$env:USERPROFILE\.fleethud\daemon.json"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$IconPath = Join-Path $ScriptDir "fleethud.ico"
if (-not (Test-Path $IconPath)) {
    $IconPath = "$env:LOCALAPPDATA\FleetHUD\fleethud.ico"
}

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32 {
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
}
"@

function Get-ConsoleUrl {
    if (Test-Path $DaemonInfoPath) {
        try {
            $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
            return "http://127.0.0.1:$($info.port)/?token=$($info.token)"
        } catch {}
    }
    return "http://127.0.0.1:$Port/"
}

function Find-FleetHUDWindow {
    $procs = Get-Process -Name "msedge", "chrome" -ErrorAction SilentlyContinue
    foreach ($p in $procs) {
        if ($p.MainWindowTitle -like "*FleetHUD*" -and $p.MainWindowHandle -ne [IntPtr]::Zero) {
            return $p.MainWindowHandle
        }
    }
    return [IntPtr]::Zero
}

function Open-Console {
    $existingHwnd = Find-FleetHUDWindow
    if ($existingHwnd -ne [IntPtr]::Zero) {
        if ([Win32]::IsIconic($existingHwnd)) {
            [Win32]::ShowWindow($existingHwnd, 9) | Out-Null # SW_RESTORE
        }
        [Win32]::ShowWindow($existingHwnd, 5) | Out-Null # SW_SHOW
        [Win32]::SetForegroundWindow($existingHwnd) | Out-Null
        return
    }

    $url = Get-ConsoleUrl
    $localAppData = [Environment]::GetFolderPath("LocalApplicationData")
    $profileArg = "--user-data-dir=`"$localAppData\FleetHUD\AppProfile`""
    $windowArg = "--window-name=FleetHUD"
    $candidateExes = @(
        "$env:ProgramFiles (x86)\Microsoft\Edge\Application\msedge.exe",
        "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe",
        "msedge.exe",
        "$env:ProgramFiles\Google\Chrome\Application\chrome.exe",
        "$env:ProgramFiles (x86)\Google\Chrome\Application\chrome.exe",
        "chrome.exe"
    )

    $launched = $false
    foreach ($exe in $candidateExes) {
        if (Test-Path $exe -PathType Leaf -ErrorAction SilentlyContinue) {
            Start-Process $exe -ArgumentList "--app=$url $profileArg $windowArg --window-size=1280,850 --no-first-run --no-default-browser-check"
            $launched = $true
            break
        }
    }
    if (-not $launched) {
        Start-Process $url
    }
}

$notifyIcon = New-Object System.Windows.Forms.NotifyIcon
if (Test-Path $IconPath) {
    try {
        $notifyIcon.Icon = New-Object System.Drawing.Icon($IconPath)
    } catch {
        $notifyIcon.Icon = [System.Drawing.SystemIcons]::Application
    }
} else {
    $notifyIcon.Icon = [System.Drawing.SystemIcons]::Application
}
$notifyIcon.Text = "FleetHUD Mission Control"
$notifyIcon.Visible = $true

function Update-TrayIconBadge([int]$count) {
    try {
        $bmp = New-Object System.Drawing.Bitmap 32, 32
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit

        # White squircle background
        $whiteBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 255, 255))
        $g.FillEllipse($whiteBrush, 1, 1, 30, 30)

        # Center navy diamond
        $navyBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(15, 23, 42))
        $points = @(
            (New-Object System.Drawing.Point 16, 5),
            (New-Object System.Drawing.Point 27, 16),
            (New-Object System.Drawing.Point 16, 27),
            (New-Object System.Drawing.Point 5, 16)
        )
        $g.FillPolygon($navyBrush, $points)

        if ($count -gt 0) {
            # Green badge for active agents
            $greenBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(34, 197, 94))
            $g.FillEllipse($greenBrush, 15, 15, 16, 16)
            $badgeFont = New-Object System.Drawing.Font("Arial", 8, [System.Drawing.FontStyle]::Bold)
            $badgeTextBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(15, 23, 42))
            $sf = New-Object System.Drawing.StringFormat
            $sf.Alignment = [System.Drawing.StringAlignment]::Center
            $sf.LineAlignment = [System.Drawing.StringAlignment]::Center
            $textStr = if ($count -gt 9) { "9+" } else { "$count" }
            $g.DrawString($textStr, $badgeFont, $badgeTextBrush, (New-Object System.Drawing.RectangleF 15, 15, 16, 16), $sf)
        }

        $hIcon = $bmp.GetHicon()
        $notifyIcon.Icon = [System.Drawing.Icon]::FromHandle($hIcon)
        $g.Dispose()
        $bmp.Dispose()
    } catch {}
}

$contextMenu = New-Object System.Windows.Forms.ContextMenuStrip

$openItem = $contextMenu.Items.Add("Open Mission Control")
$openItem.Font = New-Object System.Drawing.Font($openItem.Font, [System.Drawing.FontStyle]::Bold)
$openItem.Add_Click({ Open-Console })

$sessionsItem = $contextMenu.Items.Add("Active Sessions: 0")
$sessionsItem.Enabled = $false

$spendItem = $contextMenu.Items.Add("Spend Today: `$0.00")
$spendItem.Enabled = $false

$pinItem = $contextMenu.Items.Add("Pin FleetHUD to Taskbar Tray")
$pinItem.Add_Click({
    $notifyIcon.ShowBalloonTip(4000, "How to Pin FleetHUD", "Click the ^ arrow on your Windows taskbar, then click and drag the FleetHUD icon into your visible taskbar tray.", [System.Windows.Forms.ToolTipIcon]::Info)
})

$contextMenu.Items.Add("-") | Out-Null

$exitItem = $contextMenu.Items.Add("Quit FleetHUD")
$exitItem.Add_Click({
    $notifyIcon.Visible = $false
    # Kill daemon process if running
    if (Test-Path $DaemonInfoPath) {
        try {
            $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
            Stop-Process -Id $info.pid -Force -ErrorAction SilentlyContinue
        } catch {}
    }
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

# Show balloon tip on startup
$notifyIcon.ShowBalloonTip(3000, "FleetHUD Mission Control", "FleetHUD is running in your system tray.`nTip: Drag the icon from ^ overflow to keep it pinned.", [System.Windows.Forms.ToolTipIcon]::Info)

# Polling timer for live status & badge update
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 3000
$timer.Add_Tick({
    try {
        if (Test-Path $DaemonInfoPath) {
            $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
            $res = Invoke-RestMethod -Uri "http://127.0.0.1:$($info.port)/api/v1/live?range=today&page=1&limit=1" -TimeoutSec 1 -ErrorAction SilentlyContinue
            if ($res) {
                $count = $res.active_count
                $spend = if ($res.timeline -and $res.timeline.total_cost_usd) { [double]$res.timeline.total_cost_usd } else { 0.0 }
                $sessionsItem.Text = "Active Sessions: $count"
                $spendItem.Text = "Spend Today: `$$($spend.ToString('0.00'))"

                Update-TrayIconBadge $count

                if ($count -gt 0) {
                    $notifyIcon.Text = "FleetHUD: $count Active | Spend: `$$($spend.ToString('0.00'))"
                } else {
                    $notifyIcon.Text = "FleetHUD: Fleet Idle | Spend: `$$($spend.ToString('0.00'))"
                }
            }
        }
    } catch {}
})
$timer.Start()

[System.Windows.Forms.Application]::Run()
