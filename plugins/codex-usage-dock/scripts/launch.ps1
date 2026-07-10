$ErrorActionPreference = "Stop"

$candidates = @(
  (Join-Path $env:LOCALAPPDATA "Codex Usage Dock\\codex-usage-dock.exe"),
  (Join-Path $env:LOCALAPPDATA "Programs\\Codex Usage Dock\\codex-usage-dock.exe"),
  (Join-Path $env:ProgramFiles "Codex Usage Dock\\codex-usage-dock.exe")
)

$app = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if ($app) {
  Start-Process $app
  exit 0
}

Write-Error "Codex Usage Dock is not installed. Download it from https://github.com/Nossen/codex-usage-dock/releases/latest"
exit 2
