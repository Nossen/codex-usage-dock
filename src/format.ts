export function displayPercent(value: number | null): string {
  return value === null ? "—" : `${Math.round(value)}%`;
}

export function formatResetTime(
  resetsAt: number | null,
  nowSeconds = Date.now() / 1000,
  language: "zh" | "en" = "en",
): string {
  if (resetsAt === null) return language === "zh" ? "等待 Codex 数据" : "Waiting for Codex";

  const remainingMinutes = Math.max(
    0,
    Math.ceil((resetsAt - nowSeconds) / 60),
  );
  if (remainingMinutes === 0) return language === "zh" ? "正在重置" : "Resetting now";
  if (remainingMinutes < 60) {
    return language === "zh"
      ? `${remainingMinutes} 分钟后重置`
      : `Resets in ${remainingMinutes}m`;
  }

  const hours = Math.floor(remainingMinutes / 60);
  const minutes = remainingMinutes % 60;
  if (hours < 24) {
    return language === "zh"
      ? `${hours} 小时${minutes > 0 ? ` ${minutes} 分钟` : ""}后重置`
      : `Resets in ${hours}h${minutes > 0 ? ` ${minutes}m` : ""}`;
  }

  const days = Math.floor(hours / 24);
  const remainingHours = hours % 24;
  return language === "zh"
    ? `${days} 天${remainingHours > 0 ? ` ${remainingHours} 小时` : ""}后重置`
    : `Resets in ${days}d${remainingHours > 0 ? ` ${remainingHours}h` : ""}`;
}

export function remainingPercent(usedPercent: number | null): number | null {
  return usedPercent === null ? null : Math.max(0, Math.min(100, 100 - usedPercent));
}

export function usageTone(remaining: number | null): "calm" | "watch" | "limit" {
  if (remaining === null || remaining > 30) return "calm";
  if (remaining > 10) return "watch";
  return "limit";
}
