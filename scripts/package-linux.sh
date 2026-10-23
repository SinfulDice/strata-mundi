#!/usr/bin/env bash
# Usage: package-linux.sh <version>. Builds dist/strata-<version>-linux-x86_64.tar.gz and the AppImage.
set -euo pipefail
version=$1
app_id=io.github.sinfuldice.StrataMundi
stage=dist/strata-linux
rm -rf "$stage" && mkdir -p "$stage"
cp target/release/strata README.md LICENSE LICENSE-CONTENT "$stage/"
[ -d assets ] && cp -r assets "$stage/"
tar -czf "dist/strata-$version-linux-x86_64.tar.gz" -C dist strata-linux

appdir=dist/Strata.AppDir
rm -rf "$appdir" && mkdir -p "$appdir/usr/bin"
cp -r "$stage/." "$appdir/usr/bin/"
cp "packaging/linux/$app_id.desktop" "$appdir/"
cp packaging/icons/strata.png "$appdir/$app_id.png"
ln -sf usr/bin/strata "$appdir/AppRun"
tool=dist/appimagetool
if [ ! -x "$tool" ]; then
  curl -fsSL -o "$tool" https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x "$tool"
fi
ARCH=x86_64 "$tool" --appimage-extract-and-run "$appdir" "dist/strata-$version-x86_64.AppImage"
