$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
$source=[IO.File]::ReadAllText((Join-Path $PSScriptRoot '../scripts/tray.ps1'))
$parseErrors=$null; $parseTokens=$null
$ast=[System.Management.Automation.Language.Parser]::ParseInput($source,[ref]$parseTokens,[ref]$parseErrors)
if ($parseErrors.Count -gt 0) { throw 'Tray script contains a PowerShell syntax error' }
$trayTypes=$ast.FindAll({param($node) $node -is [System.Management.Automation.Language.CommandAst] -and $node.GetCommandName() -eq 'Add-Type' -and $node.Extent.Text.Contains('public class Win32')},$true) | Select-Object -First 1
if (-not $trayTypes) { throw 'Tray native type initialization absent' }
Invoke-Expression $trayTypes.Extent.Text
if (-not ('Win32' -as [type]) -or -not ('DarkMenuColorTable' -as [type])) { throw 'Actual tray native types did not load' }
$fn=[regex]::Match($source,'(?s)function Update-TrayIconBadge\(.*?\n}\r?\n').Value
if (!$fn) { throw 'Badge function absent' }
Invoke-Expression ($fn.Replace('catch {}','catch { throw }'))
$notifyIcon=New-Object System.Windows.Forms.NotifyIcon
$baseIcon=New-Object System.Drawing.Icon((Join-Path $PSScriptRoot '../scripts/gnomcode.ico'),32,32)
$baseBitmap=$baseIcon.ToBitmap()
$corner=$baseBitmap.GetPixel(0,0)
if ($corner.R -ne 7 -or $corner.G -ne 7 -or $corner.B -ne 8 -or $corner.A -ne 255) { throw 'Windows icon must have an opaque brand-black canvas' }
$redPixels=@(for ($y=0;$y -lt 32;$y++) { for ($x=0;$x -lt 32;$x++) { $pixel=$baseBitmap.GetPixel($x,$y); if ($pixel.R -gt 180 -and $pixel.G -lt 80 -and $pixel.B -lt 120) { [pscustomobject]@{X=$x;Y=$y} } } })
$boundsX=$redPixels | Measure-Object X -Minimum -Maximum
$boundsY=$redPixels | Measure-Object Y -Minimum -Maximum
if ($redPixels.Count -eq 0 -or $boundsX.Minimum -lt 8 -or $boundsY.Minimum -lt 8 -or ($boundsX.Maximum-$boundsX.Minimum+1) -gt 17 -or ($boundsY.Maximum-$boundsY.Minimum+1) -gt 17) { throw 'Windows mark must use the centered 0.65x asset scale' }
$baseBitmap.Dispose()
$baseIcon.Dispose()
$baseIcon=New-Object System.Drawing.Icon((Join-Path $PSScriptRoot '../scripts/gnomcode-tray.ico'),32,32)
$trayBitmap=$baseIcon.ToBitmap()
$trayPixel=$trayBitmap.GetPixel(6,6)
if ($trayPixel.R -lt 180 -or $trayPixel.G -gt 80) { throw 'Tray mark must retain its full original scale' }
$trayBitmap.Dispose()
$badgeIcon=$null
Update-TrayIconBadge 1
if (!$notifyIcon.Icon) {throw 'No generated badge'}
$bitmap=$notifyIcon.Icon.ToBitmap(); $bitmap.Dispose()
Update-TrayIconBadge 0
$notifyIcon.Dispose()
$baseIcon.Dispose()
Write-Output 'PASS: native tray badge renders and returns to base icon'

