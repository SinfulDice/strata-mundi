#!/usr/bin/env bash
# System packages needed to build Strata Mundi and render headless with Mesa lavapipe.
set -euo pipefail
packages=(mesa-vulkan-drivers vulkan-tools libvulkan-dev pkg-config libudev-dev
  libasound2-dev libxkbcommon-dev libxkbcommon-x11-0 libwayland-dev libx11-dev)
missing=()
for package in "${packages[@]}"; do
  dpkg -s "$package" >/dev/null 2>&1 || missing+=("$package")
done
if [ ${#missing[@]} -gt 0 ]; then
  sudo_cmd=""
  [ "$(id -u)" -ne 0 ] && sudo_cmd="sudo"
  $sudo_cmd apt-get update
  $sudo_cmd apt-get install -y --no-install-recommends "${missing[@]}"
fi
