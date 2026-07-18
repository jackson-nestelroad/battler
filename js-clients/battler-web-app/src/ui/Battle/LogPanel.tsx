import { useState, useEffect, useRef } from "react";
import type { UiLogEntry } from "battler-state";
import Tabs from "../Common/Tabs";

import styles from "./LogPanel.module.scss";

interface LogPanelProps {
  visibleLogs: string[];
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
    <div className={`${styles.logPanel} ${isCollapsed ? styles.collapsed : ""}`}>
      <header className={styles.header}>
        <div className={styles.headerTitleRow}>
          <button
            type="button"
            className={styles.collapseToggle}
            onClick={() => setIsCollapsed(!isCollapsed)}
            title={isCollapsed ? "Expand Log Panel" : "Collapse Log Panel"}
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
          <div className={styles.engineList}>
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
          <div className={styles.logsList}>
            {visibleLogs.map((log, index) => (
              <div key={index} className={styles.logLine}>
                <span className={styles.indicator}>&gt;</span>
                <span className={styles.text}>{log}</span>
              </div>
            ))}
            {visibleLogs.length === 0 && <p className={styles.emptyLogs}>Waiting</p>}
          </div>
        )}
      </div>
    </div>
  );
}
