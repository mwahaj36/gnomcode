# FleetHUD Windows System Tray Service — https://fleethud.vercel.app
$trayMutex = New-Object System.Threading.Mutex($false, "Local\FleetHUDTray")
try {
    $ownsTray = $trayMutex.WaitOne(0)
} catch [System.Threading.AbandonedMutexException] {
    $ownsTray = $true
}
if (-not $ownsTray) {
    $trayMutex.Dispose()
    return
}

# Older tray scripts predate the mutex and can survive an upgrade.
try {
    Get-CimInstance Win32_Process -Filter "Name = 'powershell.exe'" -ErrorAction Stop |
        Where-Object { $_.ProcessId -ne $PID -and $_.CommandLine -like '*FleetHUD*tray.ps1*' } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
} catch {}

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
    [DllImport("user32.dll")] public static extern bool DestroyIcon(IntPtr hIcon);
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
$baseIcon = $null
$badgeIcon = $null
if (Test-Path $IconPath) {
    try {
        $baseIcon = New-Object System.Drawing.Icon($IconPath, 32, 32)
        $notifyIcon.Icon = $baseIcon
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
        if ($count -eq 0 -and $baseIcon) {
            $notifyIcon.Icon = $baseIcon
            if ($badgeIcon) { $badgeIcon.Dispose(); $script:badgeIcon = $null }
            return
        }

        # Draw base icon with active agent count badge
        $bmp = New-Object System.Drawing.Bitmap 32, 32
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit

        if ($baseIcon) {
            $g.DrawIcon($baseIcon, 0, 0)
        } else {
            $whiteBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 255, 255))
            $g.FillEllipse($whiteBrush, 2, 2, 28, 28)
        }

        if ($count -gt 0) {
            # Crisp green circle badge in lower-right
            $greenBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(34, 197, 94))
            $darkBorderPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(13, 17, 23), 1.5)
            $badgeRect = New-Object System.Drawing.Rectangle 14, 14, 17, 17
            $g.FillEllipse($greenBrush, $badgeRect)
            $g.DrawEllipse($darkBorderPen, $badgeRect)

            $badgeFont = New-Object System.Drawing.Font("Segoe UI", 7.5, [System.Drawing.FontStyle]::Bold)
            $badgeTextBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(13, 17, 23))
            $sf = New-Object System.Drawing.StringFormat
            $sf.Alignment = [System.Drawing.StringAlignment]::Center
            $sf.LineAlignment = [System.Drawing.StringAlignment]::Center
            $textStr = if ($count -gt 9) { "9+" } else { "$count" }
            $g.DrawString($textStr, $badgeFont, $badgeTextBrush, (New-Object System.Drawing.RectangleF 14, 14, 17, 17), $sf)
        }

        $hIcon = $bmp.GetHicon()
        $newIcon = ([System.Drawing.Icon]::FromHandle($hIcon)).Clone()
        [Win32]::DestroyIcon($hIcon) | Out-Null
        $notifyIcon.Icon = $newIcon
        if ($badgeIcon) { $badgeIcon.Dispose() }
        $script:badgeIcon = $newIcon
        $g.Dispose()
        $bmp.Dispose()
    } catch {}
}

$contextMenu = New-Object System.Windows.Forms.ContextMenuStrip
$contextMenu.ShowImageMargin = $false
$contextMenu.BackColor = [System.Drawing.Color]::FromArgb(13, 17, 23)
$contextMenu.ForeColor = [System.Drawing.Color]::White
$contextMenu.Font = New-Object System.Drawing.Font("Segoe UI", 9.0)

# 1. Open Console
$openItem = $contextMenu.Items.Add("Open Mission Control")
$openItem.Font = New-Object System.Drawing.Font($openItem.Font, [System.Drawing.FontStyle]::Bold)
$openItem.Add_Click({ Open-Console })

# Separator
$contextMenu.Items.Add("-") | Out-Null

# Active Sessions Header & dynamic list
$sessionsHeader = $contextMenu.Items.Add("Active Sessions (0)")
$sessionsHeader.Enabled = $false
$sessionsHeader.Font = New-Object System.Drawing.Font("Segoe UI", 8.0, [System.Drawing.FontStyle]::Bold)
$sessionsHeader.ForeColor = [System.Drawing.Color]::FromArgb(158, 170, 184)

# Placeholder item when idle
$idleSessionItem = $contextMenu.Items.Add("  Fleet Idle (no active agents)")
$idleSessionItem.Enabled = $false

# 5 Slots for active sessions
$sessionMenuItems = @()
for ($i = 0; $i -lt 5; $i++) {
    $item = $contextMenu.Items.Add("")
    $item.Visible = $false
    $sessionMenuItems += $item
}

# Separator
$contextMenu.Items.Add("-") | Out-Null

# Metrics
$spendItem = $contextMenu.Items.Add("API-equivalent today: `$0.00")
$spendItem.Enabled = $false
$spendItem.ForeColor = [System.Drawing.Color]::FromArgb(158, 170, 184)

# Copy local bearer token
$copyTokenItem = $contextMenu.Items.Add("Copy Local Bearer Token")
$copyTokenItem.Add_Click({
    if (Test-Path $DaemonInfoPath) {
        try {
            $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
            [System.Windows.Forms.Clipboard]::SetText($info.token)
            $notifyIcon.ShowBalloonTip(2000, "Token Copied", "Bearer token copied to clipboard for local pairing.", [System.Windows.Forms.ToolTipIcon]::Info)
        } catch {}
    }
})

# Pin FleetHUD explanation
$pinItem = $contextMenu.Items.Add("Pin FleetHUD to Taskbar Tray")
$pinItem.Add_Click({
    [System.Windows.Forms.MessageBox]::Show(
        "To keep FleetHUD pinned to your taskbar:`n`n1. Click the '^' overflow arrow next to your system clock.`n2. Click and drag the FleetHUD icon into your visible system tray.`n`nWindows will now keep FleetHUD permanently visible.",
        "Pin FleetHUD to Taskbar",
        [System.Windows.Forms.MessageBoxButtons]::OK,
        [System.Windows.Forms.MessageBoxIcon]::Information
    ) | Out-Null
})

$contextMenu.Items.Add("-") | Out-Null

# Exit
$exitItem = $contextMenu.Items.Add("Quit FleetHUD")
$exitItem.Add_Click({
    $notifyIcon.Visible = $false
    if (Test-Path $DaemonInfoPath) {
        try {
            $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
            Stop-Process -Id $info.pid -Force -ErrorAction SilentlyContinue
        } catch {}
    }
    [System.Windows.Forms.Application]::Exit()
})

$notifyIcon.ContextMenuStrip = $contextMenu
$contextMenu.Items | ForEach-Object { $_.BackColor = $contextMenu.BackColor }
foreach ($item in $sessionMenuItems) {
    $item.Add_Click({
        param($itemSender, $itemArgs)
        try {
            $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
            $sid = [uri]::EscapeDataString([string]$itemSender.Tag)
            Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$($info.port)/api/v1/sessions/$sid/focus" -Headers @{ "Authorization" = "Bearer $($info.token)" } -ErrorAction Stop | Out-Null
        } catch {}
        Open-Console
    })
}
$notifyIcon.Add_DoubleClick({ Open-Console })
$notifyIcon.Add_Click({
    param($sender, $e)
    if ($e.Button -eq [System.Windows.Forms.MouseButtons]::Left) {
        Open-Console
    }
})

$lastNotifiedApprovals = @{}

# Polling timer for live status, sessions & badge update
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 3000
$timer.Add_Tick({
    try {
        if (-not (Test-Path $DaemonInfoPath)) { return }
        $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
        $headers = @{ "Authorization" = "Bearer $($info.token)" }

        # Query live metrics & active sessions
        $live = Invoke-RestMethod -Uri "http://127.0.0.1:$($info.port)/api/v1/live?range=today&status=active&page=1&limit=5" -Headers $headers -TimeoutSec 1 -ErrorAction SilentlyContinue
        if (-not $live) { return }

        $count = $live.active_count
        $spend = if ($live.timeline -and $live.timeline.total_cost_usd) { [double]$live.timeline.total_cost_usd } else { 0.0 }
        $spendItem.Text = "API-equivalent today: `$$($spend.ToString('0.00'))"
        $sessionsHeader.Text = "Active Sessions ($count)"

        Update-TrayIconBadge $count

        if ($count -gt 0) {
            $notifyIcon.Text = "FleetHUD: [RUN] $count Active | `$$($spend.ToString('0.00'))"
        } else {
            $notifyIcon.Text = "FleetHUD: [IDLE] Ready | `$$($spend.ToString('0.00'))"
        }

        # The live response already includes the first five sessions, active first.
        $activeList = @($live.sessions | Where-Object { $_.is_active })

        if ($activeList.Count -eq 0) {
            $idleSessionItem.Visible = $true
            for ($i = 0; $i -lt 5; $i++) {
                $sessionMenuItems[$i].Visible = $false
            }
        } else {
            $idleSessionItem.Visible = $false
            for ($i = 0; $i -lt 5; $i++) {
                if ($i -lt $activeList.Count) {
                    $s = $activeList[$i]
                    $badge = if ($s.state -eq "awaiting_approval") { "[WAIT]" } else { "[RUN]" }
                    $proj = if ($s.project_name -and $s.project_name -ne "unknown") { $s.project_name } else { "fleet" }
                    $stateText = $s.state
                    $menuText = "  $badge $proj | $($s.agent) ($stateText)"

                    $sessionMenuItems[$i].Text = $menuText
                    $sessionMenuItems[$i].Visible = $true
                    $sessionMenuItems[$i].Tag = $s.id

                    # Trigger approval notification once
                    if ($s.state -eq "awaiting_approval" -and -not $lastNotifiedApprovals.ContainsKey($s.id)) {
                        $lastNotifiedApprovals[$s.id] = [DateTime]::UtcNow
                        $notifyIcon.ShowBalloonTip(5000, "$($s.agent) needs approval", "Session in $proj is waiting for your approval to proceed.", [System.Windows.Forms.ToolTipIcon]::Warning)
                    }
                } else {
                    $sessionMenuItems[$i].Visible = $false
                }
            }
        }
    } catch {}
})
$timer.Start()

[System.Windows.Forms.Application]::Run()
$notifyIcon.Dispose()
if ($badgeIcon) { $badgeIcon.Dispose() }
if ($baseIcon) { $baseIcon.Dispose() }
$trayMutex.ReleaseMutex()
$trayMutex.Dispose()
