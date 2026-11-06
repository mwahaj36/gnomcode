Set WshShell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
scriptDir = fso.GetParentFolderName(WScript.ScriptFullName)

desktop = fso.BuildPath(scriptDir, "desktop\gnomcode-desktop.exe")
If fso.FileExists(desktop) Then
    WshShell.Run """" & desktop & """ --background", 0, False
Else
' Start background daemon
WshShell.Run """" & scriptDir & "\gnomcode.exe"" daemon", 0, False

' Start system tray icon
ps1Path = scriptDir & "\tray.ps1"
WshShell.Run "powershell.exe -WindowStyle Hidden -ExecutionPolicy Bypass -NoProfile -File """ & ps1Path & """", 0, False
End If
