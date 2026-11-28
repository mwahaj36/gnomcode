# Builds .github/assets/console-demo.gif from frames captured by web/e2e/demo.record.ts.
# Usage: (in web/) npx playwright test -c playwright.demo.config.ts; then run this script. Needs ffmpeg on PATH.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$frames = Join-Path $root 'target/demo-frames'
$out = Join-Path $root '.github/assets/console-demo.gif'
$filter = 'scale=900:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=96[p];[b][p]paletteuse=dither=bayer:bayer_scale=4'
ffmpeg -y -loglevel error -framerate 0.5 -i (Join-Path $frames 'frame-%02d.png') -vf $filter -loop 0 $out
Get-Item $out | Select-Object Name, Length
