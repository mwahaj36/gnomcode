# Build the native Windows release, including notices and verified update archives.
param([string]$Version)
$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../../../..")).Path
Set-Location $RepoRoot
$SetupSource = Get-Content "scripts/Setup.cs" -Raw
if ($SetupSource -notmatch 'AssemblyVersion\("(\d+\.\d+\.\d+)\.0"\)') { throw "Missing installer version" }
$SourceVersion = $Matches[1]
if ($Version -and $Version -ne $SourceVersion) { throw "Bump all versions before packaging" }
$Version = $SourceVersion
$Cargo = (Get-Command cargo.exe -ErrorAction Stop).Source
$Csc = Join-Path $env:WINDIR "Microsoft.NET/Framework64/v4.0.30319/csc.exe"
if (-not (Test-Path $Csc)) { throw ".NET Framework C# compiler is required" }
$Out = Join-Path $RepoRoot "web/public/downloads"
& powershell -NoProfile -File scripts/check-tray.ps1
if ($LASTEXITCODE -ne 0) { throw "Native tray icon regression failed" }

& $Cargo build --locked -p gnomcode --release
if ($LASTEXITCODE -ne 0) { throw "Native Rust release build failed" }
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $RepoRoot "target" }
$NativeExe = Join-Path $TargetDir "release/gnomcode.exe"
if ((& $NativeExe --version) -ne "gnomcode $Version") { throw "Binary version mismatch" }
Push-Location (Join-Path $RepoRoot "web")
try {
  $env:GNOMCODE_BINARY = $NativeExe
  & npm run test:e2e
  if ($LASTEXITCODE -ne 0) { throw "Console browser tests failed against the release binary" }
} finally { Remove-Item Env:GNOMCODE_BINARY -ErrorAction SilentlyContinue; Pop-Location }
& $Cargo about generate about.hbs --output-file "$Out/THIRD_PARTY_NOTICES.html"
if ($LASTEXITCODE -ne 0) { throw "Third-party notices generation failed; install cargo-about --features cli --locked" }
$TerminalLicense = [System.Net.WebUtility]::HtmlEncode([IO.File]::ReadAllText((Join-Path $RepoRoot "web/public/vendor/XTERM-LICENSE.txt")))
[IO.File]::AppendAllText("$Out/THIRD_PARTY_NOTICES.html", "<h2>xterm.js 6.0.0 (MIT)</h2><pre>$TerminalLicense</pre>", [Text.Encoding]::UTF8)
$TerminalFitLicense = [System.Net.WebUtility]::HtmlEncode([IO.File]::ReadAllText((Join-Path $RepoRoot "web/public/vendor/XTERM-FIT-LICENSE.txt")))
[IO.File]::AppendAllText("$Out/THIRD_PARTY_NOTICES.html", "<h2>xterm addon-fit 0.11.0 (MIT)</h2><pre>$TerminalFitLicense</pre>", [Text.Encoding]::UTF8)
$MarkdownLicense = [System.Net.WebUtility]::HtmlEncode([IO.File]::ReadAllText((Join-Path $RepoRoot "web/public/vendor/MARKDOWN-LICENSE.txt")))
[IO.File]::AppendAllText("$Out/THIRD_PARTY_NOTICES.html", "<h2>Chat Markdown renderer</h2><pre>$MarkdownLicense</pre>", [Text.Encoding]::UTF8)
$FontNotices = Get-ChildItem "$RepoRoot/web/public/fonts/*-LICENSE.txt" | ForEach-Object { "<h2>$($_.BaseName)</h2><pre>$([System.Net.WebUtility]::HtmlEncode((([IO.File]::ReadAllText($_.FullName) -split "`n" | ForEach-Object { $_.TrimEnd() }) -join "`n")))</pre>" }
[IO.File]::AppendAllText("$Out/THIRD_PARTY_NOTICES.html", ($FontNotices -join "`n"), [Text.Encoding]::UTF8)
$NoticeLines = [IO.File]::ReadAllText("$Out/THIRD_PARTY_NOTICES.html") -split "`n" | ForEach-Object { $_.TrimEnd() }
[IO.File]::WriteAllText("$Out/THIRD_PARTY_NOTICES.html", ($NoticeLines -join "`n"), (New-Object Text.UTF8Encoding($false)))
Copy-Item $NativeExe "$Out/gnomcode.exe" -Force
Copy-Item "LICENSE" "$Out/LICENSE" -Force
$DesktopStaging = Join-Path $RepoRoot "apps/desktop/staging"
New-Item -ItemType Directory -Path (Join-Path $DesktopStaging "bin") -Force | Out-Null
Copy-Item $NativeExe (Join-Path $DesktopStaging "bin/gnomcode.exe") -Force
Copy-Item "LICENSE" (Join-Path $DesktopStaging "EULA.txt") -Force
Copy-Item "$Out/THIRD_PARTY_NOTICES.html" (Join-Path $DesktopStaging "THIRD_PARTY_NOTICES.html") -Force
& node apps/desktop/desktop-notices.cjs
if ($LASTEXITCODE -ne 0) { throw "Desktop notices staging failed" }
& npm run package --prefix apps/desktop
if ($LASTEXITCODE -ne 0) { throw "Electron packaging failed" }
$DesktopPackage=Join-Path $RepoRoot "apps/desktop/out/Gnomcode-win32-x64"
$DesktopZip=Join-Path $DesktopStaging "desktop.zip"
Compress-Archive -Path (Join-Path $DesktopPackage "*") -DestinationPath $DesktopZip -Force -CompressionLevel Optimal
$CompilerArgs = @(
    "/target:winexe", "/platform:x64", "/optimize+", "/win32icon:$RepoRoot/scripts/gnomcode.ico",
    "/reference:System.Windows.Forms.dll", "/reference:System.Drawing.dll", "/reference:System.IO.Compression.dll",
    "/resource:$NativeExe,gnomcode.exe",
    "/resource:$DesktopZip,desktop.zip",
    "/resource:$RepoRoot/scripts/tray.ps1,tray.ps1", "/resource:$RepoRoot/scripts/gnomcode.ico,gnomcode.ico", "/resource:$RepoRoot/scripts/gnomcode-tray.ico,gnomcode-tray.ico",
    "/resource:$RepoRoot/web/public/icons/gnomcode-512.png,gnomcode-mark.png",
    "/resource:$RepoRoot/scripts/run-tray.vbs,run-tray.vbs", "/resource:$RepoRoot/scripts/open-console.vbs,open-console.vbs",
    "/resource:$RepoRoot/scripts/open-dashboard.vbs,open-dashboard.vbs", "/resource:$RepoRoot/LICENSE,LICENSE",
    "/resource:$RepoRoot/scripts/open-folder.ps1,open-folder.ps1",
    "/resource:$RepoRoot/web/public/downloads/THIRD_PARTY_NOTICES.html,THIRD_PARTY_NOTICES.html",
    "/out:$Out/Gnomcode-Setup.exe", "$RepoRoot/scripts/Setup.cs"
)
# Framework csc parses forward slashes in source paths as compiler switches.
$CompilerArgs = @($CompilerArgs | ForEach-Object { if ($_.StartsWith('/')) { '/' + $_.Substring(1).Replace('/', '\') } else { $_.Replace('/', '\') } })
& $Csc $CompilerArgs
if ($LASTEXITCODE -ne 0) { throw "Native installer compilation failed" }
$Checker = Join-Path ([IO.Path]::GetTempPath()) ("gnomcode-check-" + [Guid]::NewGuid().ToString("N") + ".exe")
try {
    & $Csc "/out:$Checker" (Resolve-Path "$RepoRoot/scripts/check-installer-payload.cs").Path
    if ($LASTEXITCODE -ne 0) { throw "Installer verification helper compilation failed" }
    & $Checker "$Out/Gnomcode-Setup.exe" $NativeExe "$Out/THIRD_PARTY_NOTICES.html" "$Version.0"
    if ($LASTEXITCODE -ne 0) { throw "Installer payload verification failed" }
} finally { Remove-Item $Checker -Force -ErrorAction SilentlyContinue }

try {
    & $Csc /reference:System.Windows.Forms.dll /reference:System.Drawing.dll "/out:$Checker" (Resolve-Path "$RepoRoot/scripts/check-installer-options.cs").Path
    if ($LASTEXITCODE -ne 0) { throw "Installer options checker compilation failed" }
    & $Checker "$Out/Gnomcode-Setup.exe"
    if ($LASTEXITCODE -ne 0) { throw "Installer options verification failed" }
    & powershell -NoProfile -File "$RepoRoot/scripts/check-open-folder.ps1"
    if ($LASTEXITCODE -ne 0) { throw "Explorer launcher verification failed" }
} finally { Remove-Item $Checker -Force -ErrorAction SilentlyContinue }

& python scripts/package-artifacts.py --windows-exe $NativeExe --windows-setup "$Out/Gnomcode-Setup.exe"
if ($LASTEXITCODE -ne 0) { throw "Release archive and checksum generation failed" }
& npm run build --prefix web
if ($LASTEXITCODE -ne 0) { throw "Web production build failed" }
Write-Output "gnomcode v$Version release packaged with verified installer payload and SHA-256 manifests."
