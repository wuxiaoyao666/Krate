# Krate 开发说明

## PDF 功能

- 从当前版本开始，`PDF 加密/解密` 已迁移到 Rust（Tauri command）。
- 前端直接调用：`encrypt_pdf`、`decrypt_pdf`。
- 运行与打包不再依赖 Python sidecar。

## `krate_extension` 目录

- 该目录目前仅保留为历史参考，不参与主应用运行链路。
