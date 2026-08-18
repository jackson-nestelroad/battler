import type { FormattedUiLog } from "battler-log-formatter";
import type { UiLogEntry } from "battler-state";
import { useEffect, useRef, useState, Fragment } from "react";
import Tabs from "../Common/Tabs";

import styles from "./LogPanel.module.scss";

interface LogPanelProps {
  visibleLogs: FormattedUiLog[];
  uiLogs: UiLogEntry[];
  engineLogs?: string[];
}

export default function LogPanel({ visibleLogs, uiLogs, engineLogs = [] }: LogPanelProps) {
  const [mode, setMode] = useState<"text" | "json" | "engine">("text");
  const [isCollapsed, setIsCollapsed] = useState(false);
  const scrollRef = useRef<HTMLDivElement | null>(null);

  // Automatically scroll to bottom on new logs
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [visibleLogs, uiLogs, engineLogs, mode]);

  return (
    <div className={`card ${styles.logPanel} ${isCollapsed ? styles.collapsed : ""}`}>
      <header className={`card-header ${styles.header}`}>
        <div className="flex-row align-center gap-m">
          <button
            type="button"
            className={styles.collapseToggle}
            onClick={() => setIsCollapsed(!isCollapsed)}
            title={isCollapsed ? "Expand log panel" : "Collapse log panel"}
          >
            {isCollapsed ? "▲" : "▼"}
          </button>
          <h3>Logs</h3>
        </div>
        <Tabs
          active={mode}
          onChange={setMode}
          options={[
            { value: "text", label: "Text" },
            { value: "json", label: "JSON" },
            { value: "engine", label: "Engine" },
          ]}
        />
      </header>

      <div className={styles.scrollArea} ref={scrollRef}>
        {mode === "json" && (
          <pre className={styles.jsonLogs}>{JSON.stringify(uiLogs, null, 2)}</pre>
        )}

        {mode === "engine" && (
          <div className="flex-col gap-xs">
            {engineLogs.map((log, index) => (
              <div key={index} className={styles.engineLogLine}>
                <span className={styles.indicator}>#</span>
                <span className={styles.text}>{log}</span>
              </div>
            ))}
            {engineLogs.length === 0 && <p className={styles.emptyLogs}>None</p>}
          </div>
        )}

        {mode === "text" && (
          <div className="flex-col gap-s">
            {visibleLogs.map((log, index) => (
              <div key={index} className={`${styles.logLine} ${styles[log.category] || ""}`}>
                <span className={styles.indicator}>&gt;</span>
                <span className={styles.text}>
                  {log.tokens.map((token, i) => {
                    if (token.type === "text") {
                      return <Fragment key={i}>{token.value}</Fragment>;
                    }
                    const ctxVal = log.context[token.value];
                    if (typeof ctxVal === "string") return <Fragment key={i}>{ctxVal}</Fragment>;
                    if (Array.isArray(ctxVal)) {
                      return <Fragment key={i}>{ctxVal.map(v => typeof v === "string" ? v : v.text).join(", ")}</Fragment>;
                    }
                    if (ctxVal && typeof ctxVal === "object" && "text" in ctxVal) {
                      return <Fragment key={i}>{ctxVal.text}</Fragment>;
                    }
                    return <Fragment key={i}>{`{{${token.value}}}`}</Fragment>;
                  })}
                </span>
              </div>
            ))}
            {visibleLogs.length === 0 && <p className={styles.emptyLogs}>None</p>}
          </div>
        )}
      </div>
    </div>
  );
}
