#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."
uv run python main.py pdf_ops decrypt_pdf '{"input":"/tmp/demo.pdf","output":"/tmp/demo_unlocked.pdf","password":"123"}'
