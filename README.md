# Krate 开发说明

## 1. 运行项目

### Python 扩展（`krate_extension`）

1. 进入目录：

```bash
cd krate_extension
```

2. 使用 `uv` 同步依赖（替代 `pip install -r requirements.txt`）：

```bash
uv sync
```

3. 执行对应平台打包脚本：

- macOS / Linux: `bash cmd/package.sh`
- Windows: `cmd\\package.bat`

4. 将打包产物放到 `src-tauri/bin` 目录下。

## 2. 文档能力

- PDF 加密 / 解密（`pikepdf`）
- Word 转 PDF（优先走 `docx2pdf`，Linux 下自动回退到 `LibreOffice` 的 `soffice` 命令）
