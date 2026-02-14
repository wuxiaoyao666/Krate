#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

# 使用 uv 管理依赖并在其环境中执行打包
uv sync
uv run pyinstaller --noconsole --onefile \
  --add-data "modules:modules" \
  --collect-all pikepdf \
  --collect-all docx2pdf \
  --name krate_extension \
  main.py
