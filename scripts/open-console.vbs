Set WshShell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
scriptDir = fso.GetParentFolderName(WScript.ScriptFullName)

exeName = "\spawnt.exe"
If Not fso.FileExists(scriptDir & exeName) Then
    exeName = "\fleethud.exe"
End If

WshShell.Run """" & scriptDir & exeName & """ console", 0, False
