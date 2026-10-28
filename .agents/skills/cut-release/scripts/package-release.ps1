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
& node scripts/check-console.mjs
if ($LASTEXITCODE -ne 0) { throw "Embedded console JavaScript is invalid" }

& $Cargo build --locked -p fleethud --release
if ($LASTEXITCODE -ne 0) { throw "Native Rust release build failed" }
$NativeExe = Join-Path $RepoRoot "target/release/fleethud.exe"
if ((& $NativeExe --version) -ne "fleethud $Version") { throw "Binary version mismatch" }
& $Cargo about generate about.hbs --output-file "$Out/THIRD_PARTY_NOTICES.html"
if ($LASTEXITCODE -ne 0) { throw "Third-party notices generation failed; install cargo-about --features cli --locked" }
Copy-Item $NativeExe "$Out/fleethud.exe" -Force
Copy-Item "LICENSE" "$Out/LICENSE" -Force
$CompilerArgs = @(
    "/target:winexe", "/platform:x64", "/optimize+", "/win32icon:scripts/fleethud.ico",
    "/reference:System.Windows.Forms.dll", "/reference:System.Drawing.dll", "/reference:System.IO.Compression.dll",
    "/resource:target/release/fleethud.exe,fleethud.exe",
    "/resource:scripts/tray.ps1,tray.ps1", "/resource:scripts/fleethud.ico,fleethud.ico",
    "/resource:web/public/icons/spawnt-512.png,spawnt-mark.png",
    "/resource:scripts/run-tray.vbs,run-tray.vbs", "/resource:scripts/open-console.vbs,open-console.vbs",
    "/resource:scripts/open-dashboard.vbs,open-dashboard.vbs", "/resource:LICENSE,LICENSE",
    "/resource:web/public/downloads/THIRD_PARTY_NOTICES.html,THIRD_PARTY_NOTICES.html",
    "/out:web/public/downloads/FleetHUD-Setup.exe", "scripts/Setup.cs"
)
& $Csc $CompilerArgs
if ($LASTEXITCODE -ne 0) { throw "Native installer compilation failed" }
$Checker = Join-Path ([IO.Path]::GetTempPath()) ("fleethud-check-" + [Guid]::NewGuid().ToString("N") + ".exe")
try {
    & $Csc "/out:$Checker" "scripts/check-installer-payload.cs"
    if ($LASTEXITCODE -ne 0) { throw "Installer verification helper compilation failed" }
    & $Checker "$Out/FleetHUD-Setup.exe" $NativeExe "$Out/THIRD_PARTY_NOTICES.html" "$Version.0"
    if ($LASTEXITCODE -ne 0) { throw "Installer payload verification failed" }
} finally { Remove-Item $Checker -Force -ErrorAction SilentlyContinue }

$Archive = Join-Path $Out "FleetHUD-v$Version-windows-x64.zip"
$Files = @("$Out/fleethud.exe", "$Out/FleetHUD-Setup.exe", "$Out/LICENSE", "$Out/THIRD_PARTY_NOTICES.html",
    "scripts/fleethud.ico", "scripts/tray.ps1", "scripts/run-tray.vbs", "scripts/open-console.vbs", "scripts/open-dashboard.vbs")
Compress-Archive -Path $Files -DestinationPath $Archive -Force
# Include already-built Unix archives when preparing a combined release.
$Assets = @((Get-Item "$Out/fleethud.exe"), (Get-Item "$Out/FleetHUD-Setup.exe"), (Get-Item $Archive))
$Assets += @(Get-ChildItem "$Out/fleethud-v$Version-*.tar.gz" -ErrorAction SilentlyContinue)
$Rows = @($Assets | Sort-Object Name | ForEach-Object { "$( (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() )  $($_.Name)" })
$Manifest = ($Rows -join "`n") + "`n"
[IO.File]::WriteAllText("$Out/SHA256SUMS-v$Version.txt", $Manifest, [Text.Encoding]::ASCII)
[IO.File]::WriteAllText("$Out/SHA256SUMS", $Manifest, [Text.Encoding]::ASCII)
& npm run build --prefix web
if ($LASTEXITCODE -ne 0) { throw "Web production build failed" }
Write-Output "FleetHUD v$Version release packaged with verified installer payload and SHA-256 manifests."
