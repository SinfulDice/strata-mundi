# Usage: package-windows.ps1 <version>. Builds dist/strata-<version>-windows-x86_64.zip and the .msi.
param([Parameter(Mandatory)][string]$Version)
$ErrorActionPreference = "Stop"
$stage = "dist/strata-windows"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item target/release/strata.exe, README.md, LICENSE, LICENSE-CONTENT $stage
if (Test-Path assets) { Copy-Item -Recurse assets $stage }
Compress-Archive -Path "$stage/*" -DestinationPath "dist/strata-$Version-windows-x86_64.zip" -Force
# The game reads assets/ beside strata.exe: harvest the folder into the installer.
$wixBin = if ($env:WIX) { Join-Path $env:WIX "bin" } else { "" }
New-Item -ItemType Directory -Force target/wix | Out-Null
& (Join-Path $wixBin "heat.exe") dir assets -nologo -cg Assets -dr APPLICATIONFOLDER -gg -scom -sreg -sfrag `
    -var var.AssetsDir -out target/wix/assets.wxs
if ($LASTEXITCODE -ne 0) { throw "heat failed" }
cargo wix --package strata-app --no-build --include target/wix/assets.wxs -C "-dAssetsDir=assets" `
    --output "dist/strata-$Version-windows-x86_64.msi"
if ($LASTEXITCODE -ne 0) { throw "cargo wix failed" }
