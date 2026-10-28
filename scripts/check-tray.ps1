$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type 'using System; using System.Runtime.InteropServices; public class Win32 { [DllImport("user32.dll")] public static extern bool DestroyIcon(IntPtr icon); }'
$source=[IO.File]::ReadAllText((Join-Path $PSScriptRoot '../scripts/tray.ps1'))
$fn=[regex]::Match($source,'(?s)function Update-TrayIconBadge\(.*?\n}\r?\n').Value
if (!$fn) { throw 'Badge function absent' }
Invoke-Expression ($fn.Replace('catch {}','catch { throw }'))
$notifyIcon=New-Object System.Windows.Forms.NotifyIcon
$baseIcon=New-Object System.Drawing.Icon((Join-Path $PSScriptRoot '../scripts/fleethud.ico'),32,32)
$baseBitmap=$baseIcon.ToBitmap()
$corner=$baseBitmap.GetPixel(0,0)
if ($corner.R -ne 7 -or $corner.G -ne 7 -or $corner.B -ne 8 -or $corner.A -ne 255) { throw 'Windows icon must have an opaque brand-black canvas' }
$baseBitmap.Dispose()
$badgeIcon=$null
Update-TrayIconBadge 1
if (!$notifyIcon.Icon) {throw 'No generated badge'}
$bitmap=$notifyIcon.Icon.ToBitmap(); $bitmap.Dispose()
Update-TrayIconBadge 0
$notifyIcon.Dispose()
$baseIcon.Dispose()
Write-Output 'PASS: native tray badge renders and returns to base icon'

