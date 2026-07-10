export type ConnectionStatus = "connecting" | "connected" | "error";

export interface UsageWindow {
  usedPercent: number;
  windowDurationMins: number;
  resetsAt: number;
}

export interface UsageSnapshot {
  fiveHour: UsageWindow | null;
  sevenDay: UsageWindow | null;
  connection: ConnectionStatus;
  error: string | null;
  updatedAt: number | null;
}
