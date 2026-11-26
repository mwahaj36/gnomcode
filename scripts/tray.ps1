# gnomcode Windows system tray - https://gnomcode.vercel.app
$trayMutex = New-Object System.Threading.Mutex($false, "Local\GnomcodeTray")
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
        Where-Object { $_.ProcessId -ne $PID -and $_.CommandLine -like '*Gnomcode*tray.ps1*' } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
} catch {}

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$Port = 41738
$GnomcodeHome = if ($env:GNOMCODE_HOME -and $env:GNOMCODE_HOME.Trim()) { $env:GNOMCODE_HOME.Trim() } elseif ($env:SPAWNT_HOME -and $env:SPAWNT_HOME.Trim()) { $env:SPAWNT_HOME.Trim() } elseif (Test-Path (Join-Path $env:USERPROFILE '.spawnt')) { Join-Path $env:USERPROFILE '.spawnt' } else { Join-Path $env:USERPROFILE '.gnomcode' }
$DaemonInfoPath = Join-Path $GnomcodeHome "daemon.json"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$IconPath = Join-Path $ScriptDir "gnomcode-tray.ico"
if (-not (Test-Path $IconPath)) {
    $IconPath = "$env:LOCALAPPDATA\Gnomcode\gnomcode-tray.ico"
    if (-not (Test-Path $IconPath)) { $IconPath = Join-Path $ScriptDir "gnomcode.ico" }
}

Add-Type -ReferencedAssemblies System.Windows.Forms,System.Drawing -ErrorAction Stop @"
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
    public static bool Light;
    public override Color MenuBorder { get { return Light ? Color.FromArgb(190, 203, 216) : Color.FromArgb(56, 56, 62); } }
    public override Color MenuItemBorder { get { return Color.Transparent; } }
    public override Color MenuItemSelected { get { return Light ? Color.FromArgb(237, 240, 243) : Color.FromArgb(20, 20, 22); } }
    public override Color ToolStripDropDownBackground { get { return Light ? Color.FromArgb(246, 247, 248) : Color.FromArgb(13, 13, 15); } }
    public override Color ImageMarginGradientBegin { get { return Light ? Color.FromArgb(246, 247, 248) : Color.FromArgb(13, 13, 15); } }
    public override Color ImageMarginGradientMiddle { get { return Light ? Color.FromArgb(246, 247, 248) : Color.FromArgb(13, 13, 15); } }
    public override Color ImageMarginGradientEnd { get { return Light ? Color.FromArgb(246, 247, 248) : Color.FromArgb(13, 13, 15); } }
    public override Color SeparatorDark { get { return Light ? Color.FromArgb(217, 223, 230) : Color.FromArgb(36, 36, 40); } }
    public override Color SeparatorLight { get { return Light ? Color.FromArgb(246, 247, 248) : Color.FromArgb(13, 13, 15); } }
    public override Color MenuItemSelectedGradientBegin { get { return Light ? Color.FromArgb(237, 240, 243) : Color.FromArgb(20, 20, 22); } }
    public override Color MenuItemSelectedGradientEnd { get { return Light ? Color.FromArgb(237, 240, 243) : Color.FromArgb(20, 20, 22); } }
    public override Color MenuItemPressedGradientBegin { get { return Light ? Color.FromArgb(237, 240, 243) : Color.FromArgb(28, 28, 32); } }
    public override Color MenuItemPressedGradientEnd { get { return Light ? Color.FromArgb(237, 240, 243) : Color.FromArgb(28, 28, 32); } }
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

function Find-GnomcodeWindow {
    $procs = Get-Process -Name "msedge", "chrome" -ErrorAction SilentlyContinue
    foreach ($p in $procs) {
        if (($p.MainWindowTitle -like "*gnomcode*" -or $p.MainWindowTitle -like "*Gnomcode*") -and $p.MainWindowHandle -ne [IntPtr]::Zero) {
            return $p.MainWindowHandle
        }
    }
    return [IntPtr]::Zero
}

function Open-Console {
    $desktop = Join-Path $ScriptDir "desktop/gnomcode-desktop.exe"
    if(Test-Path -LiteralPath $desktop) {
        Start-Process -FilePath $desktop -WindowStyle Hidden
        return
    }
    try {
        $engine = Join-Path $ScriptDir "gnomcode.exe"
        Start-Process -FilePath $engine -ArgumentList "daemon" -WindowStyle Hidden -ErrorAction Stop
    } catch {
        [System.Windows.Forms.MessageBox]::Show("Gnomcode could not start. Reinstall the latest build from gnomcode.vercel.app/downloads.", "Gnomcode") | Out-Null
        return
    }
    $existingHwnd = Find-GnomcodeWindow
    if ($existingHwnd -ne [IntPtr]::Zero -and $script:lastConsoleUrl -eq (Get-ConsoleUrl)) {
        if ([Win32]::IsIconic($existingHwnd)) {
            [Win32]::ShowWindow($existingHwnd, 9) | Out-Null # SW_RESTORE
        }
        [Win32]::ShowWindow($existingHwnd, 5) | Out-Null # SW_SHOW
        [Win32]::SetForegroundWindow($existingHwnd) | Out-Null
        return
    }

    $url = Get-ConsoleUrl
    $script:lastConsoleUrl = $url
    $localAppData = [Environment]::GetFolderPath("LocalApplicationData")
    $profileArg = "--user-data-dir=`"$localAppData\Gnomcode\AppProfile`""
    $windowArg = "--window-name=gnomcode"
    $candidateExes = @(
        "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
        "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe",
        "msedge.exe",
        "$env:ProgramFiles\Google\Chrome\Application\chrome.exe",
        "${env:ProgramFiles(x86)}\Google\Chrome\Application\chrome.exe",
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
$notifyIcon.Text = "gnomcode Mission Control"
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
            # Square red session count matches the canonical brand mark.
            $badgeBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(79, 103, 240))
            $darkPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(7, 7, 8), 2.0)
            $badgeRect = New-Object System.Drawing.Rectangle 13, 13, 18, 18
            $g.FillRectangle($badgeBrush, $badgeRect)
            $g.DrawRectangle($darkPen, $badgeRect)

            # High-visibility bold typography
            $fontSize = if ($count -gt 9) { 7.5 } else { 9.5 }
            $badgeFont = New-Object System.Drawing.Font("Arial", $fontSize, [System.Drawing.FontStyle]::Bold)
            $badgeTextBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(7, 7, 8))
            $sf = New-Object System.Drawing.StringFormat
            $sf.Alignment = [System.Drawing.StringAlignment]::Center
            $sf.LineAlignment = [System.Drawing.StringAlignment]::Center
            $textStr = if ($count -gt 9) { "9+" } else { "$count" }
            $textY = if ($count -gt 9) { 13.5 } else { 14 }
            $g.DrawString($textStr, $badgeFont, $badgeTextBrush, (New-Object System.Drawing.RectangleF 13, $textY, 18, 18), $sf)

            $badgeBrush.Dispose()
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
$contextMenu.BackColor = [System.Drawing.Color]::FromArgb(13, 13, 15)
$contextMenu.ForeColor = [System.Drawing.Color]::White
$contextMenu.Font = New-Object System.Drawing.Font("Segoe UI", 9.0)
$contextMenu.Padding = New-Object System.Windows.Forms.Padding(2, 4, 2, 4)

# Follow the device appearance each time the menu opens, including changes while running.
$contextMenu.Add_Opening({
    $light = $false
    try { $light = (Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize').AppsUseLightTheme -eq 1 } catch {}
    [DarkMenuColorTable]::Light = $light
    $contextMenu.BackColor = if ($light) { [Drawing.Color]::FromArgb(246,247,248) } else { [Drawing.Color]::FromArgb(13,13,15) }
    $contextMenu.ForeColor = if ($light) { [Drawing.Color]::FromArgb(12,17,25) } else { [Drawing.Color]::FromArgb(246,247,248) }
    foreach ($item in $contextMenu.Items) { $item.ForeColor = $contextMenu.ForeColor }
})

# 1. Open Console
$openItem = $contextMenu.Items.Add("Open gnomcode")
$openItem.Font = New-Object System.Drawing.Font("Segoe UI", 9.0, [System.Drawing.FontStyle]::Bold)
$openItem.ForeColor = [System.Drawing.Color]::White
$openItem.Add_Click({ Open-Console })

# Separator
$contextMenu.Items.Add("-") | Out-Null

# Active Sessions Header & dynamic list
$sessionsHeader = $contextMenu.Items.Add("Active sessions (0)")
$sessionsHeader.Enabled = $false
$sessionsHeader.Font = New-Object System.Drawing.Font("Segoe UI", 8.0, [System.Drawing.FontStyle]::Bold)
$sessionsHeader.ForeColor = [System.Drawing.Color]::FromArgb(139, 148, 158)

# Placeholder item when idle
$idleSessionItem = $contextMenu.Items.Add("  No active agents")
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
$spendItem = $contextMenu.Items.Add("Today: Price unavailable")
$spendItem.Enabled = $false
$spendItem.Font = New-Object System.Drawing.Font("Segoe UI", 8.5)
$spendItem.ForeColor = [System.Drawing.Color]::FromArgb(139, 148, 158)

$contextMenu.Items.Add("-") | Out-Null

# Exit
$exitItem = $contextMenu.Items.Add("Quit gnomcode")
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

function Focus-AgentApp([string]$agent, [string]$projectName, [string]$surface) {
    try {
        $names = switch -Regex ($agent) {
            "^codex$" { if ($surface -eq 'desktop_app') { @('ChatGPT','Codex') } else { @('WindowsTerminal','cmd','powershell','pwsh') } }
            "^claude" { if ($surface -eq 'desktop_app') { @('Claude') } else { @('WindowsTerminal','cmd','powershell','pwsh') } }
            "antigravity" { @('Antigravity') }
            default { @() }
        }
        $shell = New-Object -ComObject WScript.Shell
        $windows = @(Get-Process -Name $names -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero })
        $windows = $windows | Sort-Object @{Expression={ if ($projectName -and $_.MainWindowTitle.IndexOf($projectName,[StringComparison]::OrdinalIgnoreCase) -ge 0) {0} else {1} }}
        foreach ($window in $windows) {
            if ([Win32]::IsIconic($window.MainWindowHandle)) { [Win32]::ShowWindow($window.MainWindowHandle,9) | Out-Null }
            if ($shell.AppActivate($window.Id)) { [Win32]::SetForegroundWindow($window.MainWindowHandle) | Out-Null; return $true }
        }
    } catch {}
    return $false
}

$copyWorkspaceItem = New-Object System.Windows.Forms.ToolStripMenuItem("Copy workspace path")
$contextMenu.Items.Insert([Math]::Max(0,$contextMenu.Items.Count-1),$copyWorkspaceItem)
$copyWorkspaceItem.Visible = $false
$copyWorkspaceItem.Add_Click({ if ($copyWorkspaceItem.Tag) { [System.Windows.Forms.Clipboard]::SetText([string]$copyWorkspaceItem.Tag) } })

$notifyIcon.ContextMenuStrip = $contextMenu
foreach ($item in $sessionMenuItems) {
    $item.Add_Click({
        param($itemSender, $itemArgs)
        $tag = $itemSender.Tag
        $sid = ""
        $agent = ""
        $proj = ""
        $surface = ""
        if ($tag -is [hashtable]) {
            $sid = [string]$tag.Id
            $agent = [string]$tag.Agent
            $proj = [string]$tag.Project
            $surface = [string]$tag.Surface
        } elseif ($tag -is [string]) {
            $sid = $tag
        }

        $focusResult = $null
        $copyWorkspaceItem.Visible = $false
        if ($sid) {
            try {
                $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
                $escSid = [uri]::EscapeDataString($sid)
                $focusResult = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$($info.port)/api/v1/sessions/$escSid/focus" -Headers @{ "Authorization" = "Bearer $($info.token)" } -TimeoutSec 5 -ErrorAction Stop
                if ($focusResult.focused) { return }
            } catch {}
        }

        $focused = Focus-AgentApp $agent $proj $surface
        if (-not $focused) {
            if ($focusResult.repo_path) { $copyWorkspaceItem.Tag = $focusResult.repo_path; $copyWorkspaceItem.Visible = $true }
            $notifyIcon.ShowBalloonTip(4000,"Session window unavailable","Open the agent app to continue this session.",[System.Windows.Forms.ToolTipIcon]::Info)
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

$lastNotifiedSaturation = @{}

# Polling timer for live status, sessions & badge update
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 3000
$timer.Add_Tick({
    try {
        if (-not (Test-Path $DaemonInfoPath)) { throw "Daemon unavailable" }
        $info = Get-Content $DaemonInfoPath -Raw | ConvertFrom-Json
        $headers = @{ "Authorization" = "Bearer $($info.token)" }

        # Query live metrics & active sessions
        $from = [uri]::EscapeDataString([DateTime]::Today.ToUniversalTime().ToString("o"))
        $live = Invoke-RestMethod -Uri "http://127.0.0.1:$($info.port)/api/v1/live?range=today&from=$from&status=ongoing&page=1&limit=5" -Headers $headers -TimeoutSec 3 -ErrorAction Stop
        if (-not $live) { throw "Daemon unavailable" }
        $idleSessionItem.Text = "  No ongoing local sessions"
        $count = if ($null -ne $live.ongoing_count) { $live.ongoing_count } else { $live.active_count }
        $hasSpend = $null -ne $live.timeline -and ($live.timeline.priced_turns -gt 0 -or $live.timeline.cost_basis -in @('estimated','exact','api')) -and $null -ne $live.timeline.total_cost_usd
        if ($hasSpend) {
            $spend = [double]$live.timeline.total_cost_usd
            $coverage = if ($live.timeline.unpriced_turns -gt 0) { "partial estimate" } else { "estimate" }; $spendItem.Text = "Today: `$$($spend.ToString('0.00')) $coverage"
        } else {
            $spendItem.Text = "Today: Price unavailable"
        }
        $sessionsHeader.Text = "Ongoing sessions: $count"

        Update-TrayIconBadge $count

        if ($count -gt 0) {
            $notifyIcon.Text = if ($count -eq 1) { "1 active session" } else { "$count active sessions" }
        } else {
            $notifyIcon.Text = "No active sessions"
        }

        # The live response already includes the first five sessions, active first.
        $activeList = @($live.sessions | Where-Object { $_.is_ongoing -or $_.is_active })

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
                    $badge = if ($s.state -eq "awaiting_approval") { "WAIT" } elseif ($s.state -eq "idle") { "IDLE" } else { "RUN" }
                    $proj = if ($s.project_name -and $s.project_name -ne "unknown") { $s.project_name } else { "fleet" }
                    $stateText = $s.state
                    $menuText = "  $badge | $proj | $($s.agent) | $stateText"

                    $sessionMenuItems[$i].Text = $menuText
                    $sessionMenuItems[$i].Visible = $true
                    $sessionMenuItems[$i].Tag = @{ Id = $s.id; Agent = $s.agent; Surface = $s.surface; Project = $proj }

                    # Trigger context saturation notification once when >= 90%
                    if ($s.context_window -and $s.context_window -gt 0) {
                        $ctxPct = [Math]::Round(($s.context_tokens / $s.context_window) * 100)
                        if ($ctxPct -ge 90 -and -not $lastNotifiedSaturation.ContainsKey($s.id)) {
                            $lastNotifiedSaturation[$s.id] = [DateTime]::UtcNow
                            $notifyIcon.ShowBalloonTip(5000, "$($s.agent) context saturation", "$($s.agent) in $proj is at $($ctxPct)% EST context use. Safety reserve active.", [System.Windows.Forms.ToolTipIcon]::Warning)
                        }
                    }
                } else {
                    $sessionMenuItems[$i].Visible = $false
                }
            }
        }

    } catch {
        $notifyIcon.Text = "gnomcode: daemon unavailable - click to restart"
        $sessionsHeader.Text = "Local sessions unavailable"
        $idleSessionItem.Text = "  Open Mission Control to reconnect"
        $idleSessionItem.Visible = $true
        foreach ($item in $sessionMenuItems) { $item.Visible = $false }
    }
})
$timer.Start()

[System.Windows.Forms.Application]::Run()
$notifyIcon.Dispose()
if ($badgeIcon) { $badgeIcon.Dispose() }
if ($baseIcon) { $baseIcon.Dispose() }
$trayMutex.ReleaseMutex()
$trayMutex.Dispose()
