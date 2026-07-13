import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import {
  displayPercent,
  formatResetTime,
  remainingPercent,
  usageTone,
} from "./format";
import type { UsageSnapshot, UsageWindow } from "./types";
import "./App.css";

type Language = "zh" | "en";

const COPY = {
  en: {
    title: "Usage Dock",
    subtitle: "Codex limits at a glance",
    live: "Live from Codex",
    connecting: "Connecting to Codex…",
    error: "Connection needs attention",
    session: "Session",
    fiveHours: "5 hours",
    weekly: "Weekly",
    sevenDays: "7 days",
    remaining: "remaining",
    autostart: "Launch quietly at sign-in",
    collapse: "Collapse to a small icon",
    expand: "Show usage dock",
    switchLanguage: "切换到中文",
  },
  zh: {
    title: "用量悬浮窗",
    subtitle: "Codex 额度一眼掌握",
    live: "已连接 Codex",
    connecting: "正在连接 Codex…",
    error: "连接需要处理",
    session: "五小时额度",
    fiveHours: "5 小时",
    weekly: "七天额度",
    sevenDays: "7 天",
    remaining: "剩余",
    autostart: "登录系统后静默启动",
    collapse: "收起为小图标",
    expand: "展开用量悬浮窗",
    switchLanguage: "Switch to English",
  },
} as const;

const EMPTY_SNAPSHOT: UsageSnapshot = {
  fiveHour: null,
  sevenDay: null,
  connection: "connecting",
  error: null,
  updatedAt: null,
};

interface UsageMeterProps {
  label: string;
  period: string;
  remainingLabel: string;
  language: Language;
  usage: UsageWindow | null;
}

function UsageMeter({
  label,
  period,
  remainingLabel,
  language,
  usage,
}: UsageMeterProps) {
  const percent = remainingPercent(usage?.usedPercent ?? null);
  const tone = usageTone(percent);

  return (
    <section className={`usage-meter ${tone}`}>
      <div className="meter-heading">
        <div>
          <span className="meter-label">{label}</span>
          <span className="meter-period">{period}</span>
        </div>
        <strong>{displayPercent(percent)}</strong>
      </div>
      <div className="progress-track" aria-hidden="true">
        <span style={{ width: `${percent ?? 0}%` }} />
      </div>
      <div className="meter-foot">
        <span>{remainingLabel}</span>
        <span>{formatResetTime(usage?.resetsAt ?? null, undefined, language)}</span>
      </div>
    </section>
  );
}

function initialLanguage(): Language {
  const saved = localStorage.getItem("language");
  if (saved === "zh" || saved === "en") return saved;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
}

function App() {
  const [snapshot, setSnapshot] = useState(EMPTY_SNAPSHOT);
  const [autostart, setAutostart] = useState(false);
  const [autostartBusy, setAutostartBusy] = useState(false);
  const [collapsed, setCollapsed] = useState(false);
  const [collapseBusy, setCollapseBusy] = useState(false);
  const [language, setLanguage] = useState<Language>(initialLanguage);
  const copy = COPY[language];

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;

    void invoke<UsageSnapshot>("get_usage_snapshot")
      .then((value) => mounted && setSnapshot(value))
      .catch((error) =>
        mounted &&
        setSnapshot((value) => ({
          ...value,
          connection: "error",
          error: String(error),
        })),
      );

    void listen<UsageSnapshot>("usage-updated", (event) => {
      if (mounted) setSnapshot(event.payload);
    }).then((stop) => {
      if (mounted) unlisten = stop;
      else stop();
    });

    void isAutostartEnabled()
      .then(async (enabled) => {
        if (
          import.meta.env.PROD &&
          localStorage.getItem("autostart-configured") === null
        ) {
          await enableAutostart();
          localStorage.setItem("autostart-configured", "true");
          enabled = true;
        }
        if (mounted) setAutostart(enabled);
      })
      .catch(() => undefined);

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  async function toggleAutostart() {
    if (autostartBusy) return;
    setAutostartBusy(true);
    try {
      if (autostart) await disableAutostart();
      else await enableAutostart();
      localStorage.setItem("autostart-configured", "true");
      setAutostart(!autostart);
    } finally {
      setAutostartBusy(false);
    }
  }

  function toggleLanguage() {
    const next = language === "zh" ? "en" : "zh";
    localStorage.setItem("language", next);
    setLanguage(next);
  }

  async function toggleCollapsed() {
    if (collapseBusy) return;
    const next = !collapsed;
    setCollapseBusy(true);
    try {
      await invoke("set_panel_collapsed", { collapsed: next });
      setCollapsed(next);
    } finally {
      setCollapseBusy(false);
    }
  }

  if (collapsed) {
    return (
      <button
        className="dock-bubble"
        type="button"
        aria-label={copy.expand}
        title={copy.expand}
        disabled={collapseBusy}
        onClick={() => void toggleCollapsed()}
      >
        <span className="brand-mark" aria-hidden="true">
          <i />
          <i />
        </span>
        <span className={`bubble-status ${snapshot.connection}`} />
      </button>
    );
  }

  return (
    <main className="dock-shell">
      <header className="dock-header" data-tauri-drag-region>
        <div className="brand" data-tauri-drag-region>
          <span className="brand-mark" aria-hidden="true">
            <i />
            <i />
          </span>
          <div data-tauri-drag-region>
            <h1>{copy.title}</h1>
            <p>{copy.subtitle}</p>
          </div>
        </div>
        <div className="header-actions">
          <button
            className="language-button"
            type="button"
            aria-label={copy.switchLanguage}
            onClick={toggleLanguage}
          >
            <span className={language === "zh" ? "active" : ""}>中</span>
            <i />
            <span className={language === "en" ? "active" : ""}>EN</span>
          </button>
          <button
            className="collapse-button"
            type="button"
            aria-label={copy.collapse}
            title={copy.collapse}
            disabled={collapseBusy}
            onClick={() => void toggleCollapsed()}
          >
            −
          </button>
        </div>
      </header>

      <div className="connection-row">
        <span className={`status-dot ${snapshot.connection}`} />
        <span>
          {snapshot.connection === "connected"
            ? copy.live
            : snapshot.connection === "error"
              ? copy.error
              : copy.connecting}
        </span>
      </div>

      <div className="meters">
        <UsageMeter
          label={copy.session}
          period={copy.fiveHours}
          remainingLabel={copy.remaining}
          language={language}
          usage={snapshot.fiveHour}
        />
        <UsageMeter
          label={copy.weekly}
          period={copy.sevenDays}
          remainingLabel={copy.remaining}
          language={language}
          usage={snapshot.sevenDay}
        />
      </div>

      {snapshot.error && <p className="error-message">{snapshot.error}</p>}

      <footer className="dock-footer">
        <span>{copy.autostart}</span>
        <button
          type="button"
          role="switch"
          aria-checked={autostart}
          className={`switch ${autostart ? "on" : ""}`}
          disabled={autostartBusy}
          onClick={() => void toggleAutostart()}
        >
          <span />
        </button>
      </footer>
    </main>
  );
}

export default App;
