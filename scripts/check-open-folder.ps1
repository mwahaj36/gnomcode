$ErrorActionPreference = 'Stop'
$errors = $null
$tokens = $null
$source = [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'open-folder.ps1'), [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw $errors[0] }
$function = $source.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Get-SpawntFolderUrl' }, $true)
. ([scriptblock]::Create($function.Extent.Text))
$url = Get-SpawntFolderUrl ([pscustomobject]@{port=41738;token='a&b'}) "C:\work\space & quote'#file"
if ($url -ne 'http://127.0.0.1:41738/?token=a%26b#sessions?folder=C%3A%5Cwork%5Cspace%20%26%20quote%27%23file') { throw "Folder/token URL encoding failed" }
foreach ($port in 0,65536) {
    $rejected = $false
    try { Get-SpawntFolderUrl ([pscustomobject]@{port=$port;token='a'}) 'C:\work' | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw 'Invalid port accepted' }
}
Write-Output 'PASS: Explorer folder URLs remain loopback and encode paths/tokens without command interpolation'
