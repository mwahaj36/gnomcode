Set WshShell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
scriptDir = fso.GetParentFolderName(WScript.ScriptFullName)
desktop = fso.BuildPath(scriptDir, "desktop\gnomcode-desktop.exe")
If fso.FileExists(desktop) Then
    WshShell.Run """" & desktop & """", 0, False
Else
    WshShell.Run """" & scriptDir & "\gnomcode.exe"" console", 0, False
End If
