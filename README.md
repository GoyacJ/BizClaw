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

## 安装与安全提示（未签名版本）

当前发布产物尚未做代码签名或公证，因此首次安装或启动时，macOS 和 Windows 都可能弹出安全提示。请仅在确认安装包来自本仓库 Release 且未被篡改时继续下面的操作。

### macOS

- 请先将应用从 `.dmg` 拖到 `/Applications`，再尝试启动。
- 如果看到“已损坏，无法打开”“来自未识别的开发者”或“Apple 无法检查其是否包含恶意软件”等提示，通常是 Gatekeeper 对未签名或未公证应用的拦截，不一定表示安装包真的损坏。
- 可以先尝试在 `系统设置 > 隐私与安全性` 中点击 `仍要打开`。
- 如果仍无法打开，可以在终端执行下面的命令，移除应用的 quarantine 扩展属性：

```bash
sudo xattr -dr com.apple.quarantine /Applications/BizClaw.app
```

- 如果应用不在 `/Applications`，请把命令中的路径替换成实际的 `.app` 路径。
- 执行完成后，再次启动应用即可。
- 不要对未知来源的应用执行这条命令，因为它会移除 macOS 的下载隔离标记。

### Windows

- 首次运行 `.msi` 或安装后的应用时，Windows 可能会显示 `Microsoft Defender SmartScreen`、`Windows protected your PC` 或 `Unknown publisher` 之类的提示。
- 如果你确认安装包来自本仓库 Release，可以在提示窗口中点击 `More info`，再点击 `Run anyway` 继续安装或启动。
- 不建议为了安装 BizClaw 而全局关闭 SmartScreen。
- 如果当前设备由公司策略管理，且系统不允许绕过 SmartScreen，请联系 IT 管理员放行，或改用后续提供的已签名版本。

### 参考

- Apple: [Safely open apps on your Mac](https://support.apple.com/en-us/102445)
- Apple: [Open a Mac app from an unknown developer](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unidentified-developer-mh40616/mac)
- Microsoft: [App & browser control in the Windows Security app](https://support.microsoft.com/en-gb/windows/app-browser-control-in-the-windows-security-app-8f68fb65-ebb4-3cfb-4bd7-ef0f376f3dc3)
