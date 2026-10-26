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
using System.Drawing;
using System.Windows.Forms;
using System.Runtime.InteropServices;

public class Win32 {
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool DestroyIcon(IntPtr hIcon);
}

public class DarkMenuColorTable : ProfessionalColorTable {
    public override Color MenuBorder { get { return Color.FromArgb(48, 54, 61); } }
    public override Color MenuItemBorder { get { return Color.Transparent; } }
    public override Color MenuItemSelected { get { return Color.FromArgb(33, 38, 45); } }
    public override Color ToolStripDropDownBackground { get { return Color.FromArgb(13, 17, 23); } }
    public override Color ImageMarginGradientBegin { get { return Color.FromArgb(13, 17, 23); } }
    public override Color ImageMarginGradientMiddle { get { return Color.FromArgb(13, 17, 23); } }
    public override Color ImageMarginGradientEnd { get { return Color.FromArgb(13, 17, 23); } }
    public override Color SeparatorDark { get { return Color.FromArgb(48, 54, 61); } }
    public override Color SeparatorLight { get { return Color.FromArgb(13, 17, 23); } }
    public override Color MenuItemSelectedGradientBegin { get { return Color.FromArgb(33, 38, 45); } }
    public override Color MenuItemSelectedGradientEnd { get { return Color.FromArgb(33, 38, 45); } }
    public override Color MenuItemPressedGradientBegin { get { return Color.FromArgb(40, 46, 54); } }
    public override Color MenuItemPressedGradientEnd { get { return Color.FromArgb(40, 46, 54); } }
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
            if ($script:activeHIcon) {
                [Win32]::DestroyIcon($script:activeHIcon) | Out-Null
                $script:activeHIcon = $null
            }
            if ($badgeIcon) { $badgeIcon.Dispose(); $script:badgeIcon = $null }
            return
        }

        # Draw base icon with active agent count badge
        $bmp = New-Object System.Drawing.Bitmap 32, 32
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::SingleBitPerPixelGridFit

        if ($baseIcon) {
            $g.DrawIcon($baseIcon, 0, 0)
        } else {
            $whiteBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 255, 255))
            $g.FillEllipse($whiteBrush, 2, 2, 28, 28)
            $whiteBrush.Dispose()
        }

        if ($count -gt 0) {
            # Large, crisp high-contrast green badge in lower-right
            $greenBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(34, 197, 94))
            $darkPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(13, 17, 23), 2.0)
            
            # 18x18 circle at (13, 13)
            $badgeRect = New-Object System.Drawing.Rectangle 13, 13, 18, 18
            $g.FillEllipse($greenBrush, $badgeRect)
            $g.DrawEllipse($darkPen, $badgeRect)

            # High-visibility bold typography
            $badgeFont = New-Object System.Drawing.Font("Arial", 9.5, [System.Drawing.FontStyle]::Bold)
            $badgeTextBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(13, 17, 23))
            $sf = New-Object System.Drawing.StringFormat
            $sf.Alignment = [System.Drawing.StringAlignment]::Center
            $sf.LineAlignment = [System.Drawing.StringAlignment]::Center
            $textStr = if ($count -gt 9) { "9+" } else { "$count" }
            $g.DrawString($textStr, $badgeFont, $badgeTextBrush, (New-Object System.Drawing.RectangleF 13, 14, 18, 18), $sf)

            $greenBrush.Dispose()
            $darkPen.Dispose()
            $badgeFont.Dispose()
            $badgeTextBrush.Dispose()
            $sf.Dispose()
        }

        $hIcon = $bmp.GetHicon()
        $newIcon = [System.Drawing.Icon]::FromHandle($hIcon)
        $notifyIcon.Icon = $newIcon
        if ($script:activeHIcon) {
            [Win32]::DestroyIcon($script:activeHIcon) | Out-Null
        }
        $script:activeHIcon = $hIcon
        if ($badgeIcon) { $badgeIcon.Dispose() }
        $script:badgeIcon = $newIcon
        $g.Dispose()
        $bmp.Dispose()
    } catch {}
}

$contextMenu = New-Object System.Windows.Forms.ContextMenuStrip
$colorTable = New-Object DarkMenuColorTable
$contextMenu.Renderer = New-Object System.Windows.Forms.ToolStripProfessionalRenderer($colorTable)
$contextMenu.ShowImageMargin = $false
$contextMenu.ShowCheckMargin = $false
$contextMenu.BackColor = [System.Drawing.Color]::FromArgb(13, 17, 23)
$contextMenu.ForeColor = [System.Drawing.Color]::White
$contextMenu.Font = New-Object System.Drawing.Font("Segoe UI", 9.0)
$contextMenu.Padding = New-Object System.Windows.Forms.Padding(2, 4, 2, 4)

# 1. Open Console
$openItem = $contextMenu.Items.Add("Open Mission Control")
$openItem.Font = New-Object System.Drawing.Font("Segoe UI", 9.0, [System.Drawing.FontStyle]::Bold)
$openItem.ForeColor = [System.Drawing.Color]::White
$openItem.Add_Click({ Open-Console })

# Separator
$contextMenu.Items.Add("-") | Out-Null

# Active Sessions Header & dynamic list
$sessionsHeader = $contextMenu.Items.Add("Active Sessions (0)")
$sessionsHeader.Enabled = $false
$sessionsHeader.Font = New-Object System.Drawing.Font("Segoe UI", 8.0, [System.Drawing.FontStyle]::Bold)
$sessionsHeader.ForeColor = [System.Drawing.Color]::FromArgb(139, 148, 158)

# Placeholder item when idle
$idleSessionItem = $contextMenu.Items.Add("  Fleet Idle (no active agents)")
$idleSessionItem.Enabled = $false
$idleSessionItem.ForeColor = [System.Drawing.Color]::FromArgb(110, 118, 129)

# 5 Slots for active sessions
$sessionMenuItems = @()
for ($i = 0; $i -lt 5; $i++) {
    $item = $contextMenu.Items.Add("")
    $item.Visible = $false
    $item.ForeColor = [System.Drawing.Color]::FromArgb(230, 237, 243)
    $sessionMenuItems += $item
}

# Separator
$contextMenu.Items.Add("-") | Out-Null

# Metrics
$spendItem = $contextMenu.Items.Add("Today: `$0.00 API-equivalent")
$spendItem.Enabled = $false
$spendItem.Font = New-Object System.Drawing.Font("Segoe UI", 8.5)
$spendItem.ForeColor = [System.Drawing.Color]::FromArgb(139, 148, 158)

$contextMenu.Items.Add("-") | Out-Null

# Exit
$exitItem = $contextMenu.Items.Add("Quit FleetHUD")
$exitItem.ForeColor = [System.Drawing.Color]::FromArgb(248, 81, 73)
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

function Focus-AgentApp([string]$agent, [string]$projectName) {
    try {
        $wshell = New-Object -ComObject Wscript.Shell
        
        # 1. Target window titles
        $titles = @()
        if ($projectName -and $projectName -ne "unknown" -and $projectName -ne "fleet") {
            $titles += $projectName
        }
        switch -Regex ($agent) {
            "antigravity" {
                $titles += @("Antigravity", "Google Antigravity")
            }
            "cursor" {
                $titles += @("Cursor")
            }
            "cline|roo" {
                $titles += @("Visual Studio Code", "Code")
            }
            "claude|codex|gemini" {
                $titles += @("Terminal", "PowerShell", "Command Prompt", "Windows Terminal", "cmd", "pwsh")
            }
            default {
                if ($agent) { $titles += $agent }
            }
        }

        foreach ($title in $titles) {
            if ($wshell.AppActivate($title)) {
                return $true
            }
        }

        # 2. Target process names
        $pNames = switch -Regex ($agent) {
            "antigravity" { @("Antigravity") }
            "cursor"      { @("Cursor") }
            "cline|roo"   { @("Code") }
            "claude|codex|gemini" { @("WindowsTerminal", "cmd", "powershell", "pwsh") }
            default       { @() }
        }

        foreach ($pName in $pNames) {
            $procs = Get-Process -Name $pName -ErrorAction SilentlyContinue
            foreach ($p in $procs) {
                if ($wshell.AppActivate($p.Id)) {
                    return $true
                }
            }
        }
    } catch {}
    return $false
}

$notifyIcon.ContextMenuStrip = $contextMenu
foreach ($item in $sessionMenuItems) {
    $item.Add_Click({
        param($itemSender, $itemArgs)
        $tag = $itemSender.Tag
        $sid = ""
        $agent = ""
        $proj = ""
        if ($tag -is [hashtable]) {
            $sid = [string]$tag.Id
            $agent = [string]$tag.Agent
            $proj = [string]$tag.Project
        } elseif ($tag -is [string]) {
            $sid = $tag
        }

        if ($sid) {
            try {
                $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
                $escSid = [uri]::EscapeDataString($sid)
                Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$($info.port)/api/v1/sessions/$escSid/focus" -Headers @{ "Authorization" = "Bearer $($info.token)" } -TimeoutSec 1 -ErrorAction SilentlyContinue | Out-Null
            } catch {}
        }

        $focused = Focus-AgentApp $agent $proj
        if (-not $focused) {
            Open-Console
        }
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
        $spendItem.Text = "Today: `$$($spend.ToString('0.00')) API-equivalent"
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
                    $sessionMenuItems[$i].Tag = @{ Id = $s.id; Agent = $s.agent; Project = $proj }

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
