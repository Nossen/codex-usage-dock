---
name: codex-usage-dock
description: Launch, install, or troubleshoot Codex Usage Dock, the local macOS and Windows companion that shows Codex five-hour and seven-day usage beside the Codex window.
---

# Codex Usage Dock

Use this skill when the user asks to open, install, update, or diagnose Codex Usage Dock.

## Launch

1. Detect the operating system.
2. From the plugin root, run the matching script:
   - macOS: `scripts/launch.sh`
   - Windows PowerShell: `scripts/launch.ps1`
3. If the script exits with status 0, tell the user the dock will appear when Codex is the foreground app.
4. If the script reports that the app is missing, direct the user to the latest GitHub release:
   `https://github.com/Nossen/codex-usage-dock/releases/latest`

Do not download or execute an installer without the user's approval.

## Diagnose

Check these in order:

1. The desktop companion is installed.
2. Codex or ChatGPT is running and signed in.
3. The local `codex` binary can run `codex app-server --listen stdio://`.
4. `CODEX_USAGE_DOCK_CODEX_BIN` points to a valid Codex binary if automatic discovery fails.
5. macOS Screen Recording permission is available if window bounds cannot be detected.

The companion reads quota data locally from Codex App Server. It must not request an OpenAI API key and must not upload usage data.

## Expected behavior

- Show five-hour and seven-day remaining percentages (`100 - usedPercent`).
- Follow the right side of the foreground Codex window.
- Hide when another application is active.
- Start quietly at system sign-in when the user leaves autostart enabled.
