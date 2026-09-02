import type { UiLogEntry } from "battler-state";
import { useEffect, useRef, useState, Fragment } from "react";
import Tabs from "../Common/Tabs";
import type { FormattedLogDisplayItem } from "../../utils/logFormatter";
import { formatNoticeText } from "../../utils/logFormatter";

import styles from "./LogPanel.module.scss";

interface LogPanelProps {
  visibleLogs: FormattedLogDisplayItem[];
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
          <div className="flex-col gap-xs">
            {visibleLogs.map((item, index) => {
              if (item.kind === "turn") {
                return (
                  <div key={index} className={styles.turnHeader}>
                    <span>Turn {item.turn}</span>
                  </div>
                );
              }

              if (item.kind === "request-split") {
                const prev = visibleLogs[index - 1];
                if (!prev || prev.kind === "turn" || prev.kind === "request-split") {
                  return null;
                }
                return <hr key={index} className={styles.requestDivider} />;
              }

              if (item.kind === "notice") {
                const noticeType = item.notice.type.toLowerCase();
                let noticeClass = styles.noticeLine;
                if (noticeType === "ability") {
                  noticeClass = `${styles.noticeLine} ${styles.abilityNotice}`;
                } else if (noticeType === "item") {
                  noticeClass = `${styles.noticeLine} ${styles.itemNotice}`;
                } else if (noticeType === "damage") {
                  noticeClass = `${styles.noticeLine} ${styles.damageNotice}`;
                } else if (noticeType === "heal") {
                  noticeClass = `${styles.noticeLine} ${styles.healNotice}`;
                }

                return (
                  <div key={index} className={noticeClass}>
                    <span className={styles.text}>{formatNoticeText(item.notice)}</span>
                  </div>
                );
              }

              const { message } = item;
              return (
                <div key={index} className={`${styles.logLine} ${styles[item.category] || ""}`}>
                  <span className={styles.indicator}>&gt;</span>
                  <span className={styles.text}>
                    {message.tokens.map((token, i) => {
                      if (token.type === "text") {
                        return <Fragment key={i}>{token.value}</Fragment>;
                      }
                      const ctxVal = message.context[token.value];
                      if (typeof ctxVal === "string") return <Fragment key={i}>{ctxVal}</Fragment>;
                      if (typeof ctxVal === "number") return <Fragment key={i}>{ctxVal.toString()}</Fragment>;
                      if (Array.isArray(ctxVal)) {
                        return (
                          <Fragment key={i}>
                            {ctxVal.map((v) => (typeof v === "string" ? v : v.text)).join(", ")}
                          </Fragment>
                        );
                      }
                      if (ctxVal && typeof ctxVal === "object" && "text" in ctxVal) {
                        return <Fragment key={i}>{ctxVal.text}</Fragment>;
                      }
                      return <Fragment key={i}>{`{{${token.value}}}`}</Fragment>;
                    })}
                  </span>
                </div>
              );
            })}
            {visibleLogs.length === 0 && <p className={styles.emptyLogs}>None</p>}
          </div>
        )}
      </div>
    </div>
  );
}

