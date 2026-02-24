# Krate 开发说明

## Python 扩展（`krate_extension`）

项目已切换为 `uv` 管理，不再使用 `pip install -r requirements.txt`。

### 1) 安装依赖

```bash
cd krate_extension
uv sync --group build
```

### 2) 本地调试

```bash
uv run python main.py pdf_ops encrypt_pdf '{"input":"/path/in.pdf","output":"/path/out.pdf","password":"123456"}'
```

### 3) 打包 sidecar

macOS / Linux:

```bash
./cmd/package.sh
```

Windows:

```bat
cmd\package.bat
```

将打包产物放到 `src-tauri/bin` 下。
