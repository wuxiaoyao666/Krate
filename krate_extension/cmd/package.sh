#!/bin/bash
set -euo pipefail

# 确保在 krate_extension 目录执行
cd "$(dirname "$0")/.."

if ! command -v uv >/dev/null 2>&1; then
  echo "未检测到 uv，请先安装: https://docs.astral.sh/uv/"
  exit 1
fi

# 安装运行依赖 + build 组依赖（PyInstaller）
uv sync --group build

# 使用 PyInstaller 打包 sidecar
pyinstaller_cmd=(
  uv run pyinstaller
  --noconfirm
  --noconsole
  --onefile
  --add-data "modules:modules"
  --collect-all pikepdf
  --name krate_extension
  main.py
)

"${pyinstaller_cmd[@]}"
