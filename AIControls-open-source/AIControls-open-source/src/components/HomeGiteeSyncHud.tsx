import type { CSSProperties } from "react";
import { useEffect, useMemo, useState } from "react";
import { createPortal } from "react-dom";
import { Link } from "react-router-dom";
import { getGiteeSyncStatus, type GiteeSyncStatus } from "../api/gitee";
import { useI18n } from "../i18n/provider";

function formatCountdown(totalSec: number): string {
  const s = Math.max(0, Math.floor(totalSec));
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${String(m).padStart(2, "0")}:${String(r).padStart(2, "0")}`;
}

export default function HomeGiteeSyncHud() {
  const { locale } = useI18n();
  const [st, setSt] = useState<GiteeSyncStatus | null>(null);

  useEffect(() => {
    const load = () => {
      void getGiteeSyncStatus().then(setSt);
    };
    load();
    const id = window.setInterval(load, 4000);
    return () => window.clearInterval(id);
  }, []);

  useEffect(() => {
    const id = window.setInterval(() => {
      setSt((prev) => (prev ? { ...prev } : prev));
    }, 1000);
    return () => window.clearInterval(id);
  }, []);

  if (typeof document === "undefined") return null;

  const miniStyle = useMemo(() => {
    // We don't have the full schedule interval from backend, so we model "time flowing"
    // as a per-minute progress bar driven by the countdown seconds.
    const now = Date.now();
    const nextMs = st?.nextAutoCheckMs ?? now;
    const remainSec = (nextMs - now) / 1000;
    const s = Math.max(0, Math.floor(remainSec));
    const p = 1 - (s % 60) / 60; // 0..1
    return { ["--mini-p" as never]: `${Math.round(p * 100)}%` } as CSSProperties;
  }, [st?.nextAutoCheckMs]);

  if (st === null) {
    return createPortal(
      <div
        className="home-gitee-sync home-gitee-sync--collapsed home-gitee-sync--muted"
        aria-live="polite"
      >
        <div
          className="home-gitee-sync__mini"
          style={miniStyle}
          title={locale === "zh" ? "云备份 --:--" : "Cloud backup --:--"}
          aria-label={locale === "zh" ? "云备份倒计时 --:--" : "Cloud backup countdown --:--"}
        />
        <div className="home-gitee-sync__head" aria-hidden>
          <span className="home-gitee-sync__title">{locale === "zh" ? "云备份" : "Cloud Backup"}</span>
        </div>
        <div className="home-gitee-sync__compact-row" aria-hidden>
          <span className="home-gitee-sync__time">--:--</span>
        </div>
      </div>,
      document.body,
    );
  }

  const remainSec = (st.nextAutoCheckMs - Date.now()) / 1000;
  const overdue = remainSec < -5;
  const countdownLabel = overdue ? (locale === "zh" ? "检查中…" : "Checking…") : formatCountdown(remainSec);

  let statusLine = locale === "zh" ? "未备份" : "Not backed up";
  if (st.lastMessage) {
    const ok = st.lastOk === true;
    const skip = st.lastMessage.includes("无变化");
    statusLine = ok
      ? skip
        ? locale === "zh"
          ? "已同步"
          : "Synced"
        : locale === "zh"
          ? "成功"
          : "Success"
      : locale === "zh"
        ? "失败"
        : "Failed";
  }

  return createPortal(
    <div
      className={`home-gitee-sync home-gitee-sync--collapsed${st.connected ? "" : " home-gitee-sync--muted"}`}
      aria-live="polite"
    >
      <div
        className="home-gitee-sync__mini"
        style={miniStyle}
        title={`${locale === "zh" ? "云备份" : "Cloud backup"} ${countdownLabel} · ${
          st.connected ? statusLine : locale === "zh" ? "未连接" : "Disconnected"
        }`}
        aria-label={`${
          locale === "zh" ? "云备份倒计时" : "Cloud backup countdown"
        } ${countdownLabel}, ${st.connected ? statusLine : locale === "zh" ? "未连接" : "Disconnected"}`}
      />
      <div className="home-gitee-sync__head">
        <span className="home-gitee-sync__title">{locale === "zh" ? "云备份" : "Cloud Backup"}</span>
        <Link to="/settings" className="home-gitee-sync__link">
          ⚙
        </Link>
      </div>
      <div className="home-gitee-sync__compact-row">
        <span
          className={`home-gitee-sync__dot${st.connected ? " home-gitee-sync__dot--ok" : ""}`}
          aria-hidden
        />
        <span className="home-gitee-sync__time">{countdownLabel}</span>
        <span className="home-gitee-sync__status">
          {st.connected ? statusLine : locale === "zh" ? "未连接" : "Disconnected"}
        </span>
      </div>
    </div>,
    document.body,
  );
}
