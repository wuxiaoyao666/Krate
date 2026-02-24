#!/bin/bash
set -euo pipefail

# 使用示例:
# ./cmd/test.sh /path/to/input.pdf /path/to/output.pdf 123456

cd "$(dirname "$0")/.."

if [ "$#" -ne 3 ]; then
  echo "用法: $0 <input.pdf> <output.pdf> <password>"
  exit 1
fi

uv run python main.py pdf_ops encrypt_pdf "{\"input\":\"$1\",\"output\":\"$2\",\"password\":\"$3\"}"
