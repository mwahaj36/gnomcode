Set WshShell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
scriptDir = fso.GetParentFolderName(WScript.ScriptFullName)

exeName = "\spawnt.exe"
If Not fso.FileExists(scriptDir & exeName) Then
    exeName = "\fleethud.exe"
End If

' Start background daemon
WshShell.Run """" & scriptDir & exeName & """ daemon", 0, False

' Start system tray icon
ps1Path = scriptDir & "\tray.ps1"
WshShell.Run "powershell.exe -WindowStyle Hidden -ExecutionPolicy Bypass -NoProfile -File """ & ps1Path & """", 0, False
