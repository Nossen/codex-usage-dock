#!/bin/sh

if [ "$(uname -s)" != "Darwin" ]; then
  echo "Codex Usage Dock: this launcher is for macOS." >&2
  exit 1
fi

if open -Ra "Codex Usage Dock"; then
  open -a "Codex Usage Dock"
  exit 0
fi

echo "Codex Usage Dock is not installed. Download it from https://github.com/Nossen/codex-usage-dock/releases/latest" >&2
exit 2
