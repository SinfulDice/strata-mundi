# Usage: package-windows.ps1 <version>. Builds dist/strata-<version>-windows-x86_64.zip and the .msi.
param([Parameter(Mandatory)][string]$Version)
$ErrorActionPreference = "Stop"
$stage = "dist/strata-windows"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item target/release/strata.exe, README.md, LICENSE, LICENSE-CONTENT $stage
if (Test-Path assets) { Copy-Item -Recurse assets $stage }
Compress-Archive -Path "$stage/*" -DestinationPath "dist/strata-$Version-windows-x86_64.zip" -Force
cargo wix --package strata-app --no-build --output "dist/strata-$Version-windows-x86_64.msi"
