# BizClaw

BizClaw 是一个内部使用的桌面客户端，用来引导安装 `openclaw`、保存本机连接配置、把 `OPENCLAW_GATEWAY_TOKEN` 写入系统安全存储，并托管 SSH 隧道与 OpenClaw Node。

## 开发

```bash
pnpm install
pnpm test
pnpm tauri:dev
```

## 配置策略

- 安装包不内置任何公司连接参数。
- 首次启动时手动填写 SSH 主机、用户、端口和显示名称。
- 非敏感配置保存到应用数据目录。
- `OPENCLAW_GATEWAY_TOKEN` 仅保存到 macOS Keychain / Windows Credential Manager。

## 发版

1. 同步更新以下三个版本号，保持完全一致：
   - `package.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
2. 提交并推送到 `main`。
3. 创建并推送 `vX.Y.Z` tag。
4. GitHub Actions 会自动构建并发布：
   - macOS `.dmg`
   - Windows `.msi`
