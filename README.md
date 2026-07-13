# Codex Usage Dock

An unofficial, privacy-first floating usage monitor for Codex on macOS and Windows.

Codex Usage Dock sits in the bottom-right corner of the active Codex window and shows the two limits people check most often:

- five-hour remaining percentage and reset countdown;
- seven-day remaining percentage and reset countdown.

It hides when you switch to another application, so it never floats over unrelated work.
Drag the header to place it somewhere else relative to Codex, or collapse it into a small bottom-right icon and click once to restore the full panel.

> 非官方 Codex 用量悬浮窗。支持 macOS 和 Windows，默认放在 Codex 右下角，可拖动，也可收起为小图标；显示五小时与七天周期的剩余百分比，切换到其他软件后自动隐藏。

![Codex Usage Dock showing remaining five-hour and seven-day quota](docs/screenshot.png)

Use the `中 / EN` control to switch languages. The choice is stored locally.

## How it works

The desktop companion launches the local `codex app-server`, initializes the documented JSONL protocol, reads `account/rateLimits/read`, and listens for `account/rateLimits/updated` notifications. It selects the overall `codex` bucket, recognizes the two windows by their server-provided durations (300 and 10,080 minutes), and displays `100 - usedPercent` as the remaining quota.

No API key is required. Usage and account data stay on the computer.

## Install

Download the latest installer from [GitHub Releases](https://github.com/Nossen/codex-usage-dock/releases/latest):

- macOS: `.dmg`
- Windows: `.exe` (NSIS) or `.msi`

### macOS：首次打开被系统拦截

当前 macOS 安装包尚未使用付费 Apple Developer ID 证书签名和公证，因此第一次打开时，macOS 可能提示“Apple 无法验证 Codex Usage Dock 是否包含可能危害 Mac 安全或泄漏隐私的恶意软件”。这表示 Apple 无法验证开发者身份，并不表示 Apple 已检测到恶意软件。

请只对从本项目 [GitHub Releases](https://github.com/Nossen/codex-usage-dock/releases/latest) 下载的安装包执行以下操作：

1. 下载名称以 `Codex.Usage.Dock` 开头、以 `.dmg` 结尾的文件。
2. 双击打开 `.dmg`，将 **Codex Usage Dock** 拖入 **Applications（应用程序）** 文件夹。
3. 在“应用程序”中双击 **Codex Usage Dock**。出现安全提示时，点击 **完成**，不要点击“移到废纸篓”。
4. 立即打开苹果菜单 ** → 系统设置 → 隐私与安全性**，然后向下滚动到 **安全性** 区域。
5. 找到关于 **Codex Usage Dock** 被阻止的提示，点击 **仍要打开**。部分 macOS 版本会先显示 **打开**，点击后再在确认窗口中选择 **仍要打开**。
6. 使用登录密码或 Touch ID 确认。应用随后会打开；以后启动时通常不需要重复这些步骤。

如果没有看到“仍要打开”按钮，请返回“应用程序”再次尝试打开 **Codex Usage Dock**，然后在一小时内回到“隐私与安全性”重试。不要关闭 Gatekeeper，也不要运行来源不明的终端命令。如果文件不是从上述 GitHub Releases 页面下载的，请删除它。

Apple 官方说明：[打开来自身份不明开发者的 Mac App](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unidentified-developer-mh40616/mac)

### macOS: If Apple cannot verify the app

The current macOS build is not signed and notarized with a paid Apple Developer ID certificate. Only override Gatekeeper for a copy downloaded from this project's [GitHub Releases](https://github.com/Nossen/codex-usage-dock/releases/latest).

1. Open the downloaded `.dmg` and drag **Codex Usage Dock** to **Applications**.
2. Try to open **Codex Usage Dock** from Applications. When macOS blocks it, click **Done** instead of **Move to Trash**.
3. Immediately open **Apple menu  → System Settings → Privacy & Security** and scroll down to **Security**.
4. Find the message about **Codex Usage Dock**, then click **Open Anyway**. On some macOS versions, click **Open** first and then confirm with **Open Anyway**.
5. Enter your login password or use Touch ID. The app should then open normally.

If the button is missing, try to open the app again and return to Privacy & Security within one hour. Do not disable Gatekeeper or run untrusted Terminal commands. See [Apple's official instructions](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unidentified-developer-mh40616/mac).

The first packaged launch enables a quiet system sign-in entry. You can turn it off from the dock.

Codex or ChatGPT must already be installed and signed in. If automatic Codex binary discovery fails, set `CODEX_USAGE_DOCK_CODEX_BIN` to the full path of the local `codex` executable.

## Install the Codex plugin

The optional plugin lets Codex launch and troubleshoot the desktop companion:

```bash
codex plugin marketplace add https://github.com/Nossen/codex-usage-dock
codex plugin add codex-usage-dock@codex-usage-dock
```

Start a new Codex task after installation so the plugin is loaded.

## Development

Prerequisites: Node.js 22+, Rust stable, and the platform requirements for Tauri 2.

```bash
npm install
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
CODEX_USAGE_DOCK_ALWAYS_VISIBLE=1 npm run tauri dev
```

`CODEX_USAGE_DOCK_ALWAYS_VISIBLE=1` keeps the panel visible for UI development. Without it, the panel appears only while Codex or ChatGPT is the foreground application.

## Release

Push a semantic version tag such as `v0.2.0`. GitHub Actions builds a universal macOS image and Windows installers, then attaches them to the release.

The current installers are unsigned. macOS Gatekeeper or Windows SmartScreen may warn users before opening them. Production distribution should add Apple Developer ID and Windows code-signing secrets.

## Privacy and permissions

- Reads quota percentages and reset timestamps from the local Codex App Server.
- Reads the foreground window process name and bounds to position the dock.
- Does not read prompts, source code, files, or conversation content.
- Does not send telemetry or usage data to this project.

On macOS, the system may request Screen Recording permission so window bounds can be read. The app does not capture or store screenshots.

## License

[MIT](LICENSE)
