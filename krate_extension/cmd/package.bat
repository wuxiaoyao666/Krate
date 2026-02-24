@echo off

where uv >nul 2>nul
if %errorlevel% neq 0 (
  echo 未检测到 uv，请先安装: https://docs.astral.sh/uv/
  exit /b 1
)

uv sync --group build
if %errorlevel% neq 0 exit /b %errorlevel%

uv run pyinstaller --noconfirm --noconsole --onefile --add-data "modules;modules" --collect-all pikepdf --name krate_extension main.py
