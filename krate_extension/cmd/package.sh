#!/bin/bash

# 确保在当前目录下运行
cd "$(dirname "$0")/.."

# 安装依赖
pip3 install -r requirements.txt

# 使用 PyInstaller 进行打包
# --onefile: 打包成单个可执行文件
# --add-data: macOS 上的格式为 "源路径:目标路径"
# --collect-all: 确保收集 pikepdf 的所有依赖
pyinstaller --noconsole --onefile \
  --add-data "modules:modules" \
  --collect-all pikepdf \
  --name krate_extension \
  main.py