import type { BattleState, UiLogEntry } from "battler-state";
import type { PlayerBattleData } from "battler-types";
import { useEffect, useRef, useState, Fragment } from "react";
import Tabs from "../Common/Tabs";
import type { ContextValue } from "battler-log-formatter";
import type { FormattedLogDisplayItem, LogDividerType } from "../../utils/logFormatter";
import { formatContextValue, formatNoticeText } from "../../utils/logFormatter";
import MonTooltipTrigger from "../Common/Tooltip/MonTooltipTrigger";
import EngineLogViewer from "./EngineLogViewer";
import PlayerStateViewer from "./PlayerStateViewer";

import styles from "./LogPanel.module.scss";

interface LogPanelProps {
  visibleLogs: FormattedLogDisplayItem[];
  uiLogs: UiLogEntry[];
  engineLogs?: string[];
  battleState?: BattleState | null;
  rules?: string[] | null;
  playerData?: PlayerBattleData | null;
  allyPlayerData?: Record<string, PlayerBattleData> | null;
  localPlayerId?: string | null;
}

function renderLogDivider(
  visibleLogs: readonly FormattedLogDisplayItem[],
  index: number,
  initialSubtype: LogDividerType,
) {
  const prev = visibleLogs[index - 1];
  if (!prev || prev.kind === "turn" || prev.kind === "divider") {
    return null;
  }

  let nextNonDivider: FormattedLogDisplayItem | undefined;
  let hasContinueInGroup = initialSubtype === "continue";
  for (let i = index + 1; i < visibleLogs.length; i++) {
    const nextItem = visibleLogs[i];
    if (nextItem.kind === "divider") {
      if (nextItem.subtype === "continue") {
        hasContinueInGroup = true;
      }
      continue;
    }
    nextNonDivider = nextItem;
    break;
  }

  if (!nextNonDivider || nextNonDivider.kind === "turn") {
    return null;
  }

  if (hasContinueInGroup) {
    return <hr key={index} className={styles.continueDivider} />;
  }
  return <div key={index} className={styles.residualDivider} aria-hidden="true" />;
}

function renderTokenValue(
  ctxVal: ContextValue | undefined,
  battleState: BattleState | null | undefined,
  key: string | number,
  rules?: string[] | null,
) {
  if (ctxVal == null) return null;

  if (
    typeof ctxVal === "object" &&
    !Array.isArray(ctxVal) &&
    "monRef" in ctxVal &&
    ctxVal.monRef
  ) {
    const text = formatContextValue(ctxVal);
    return (
      <MonTooltipTrigger
        key={key}
        monRef={ctxVal.monRef}
        battleState={battleState}
        rules={rules}
        preferredPlacement="left"
      >
        <span className={styles.monHoverTrigger}>{text}</span>
      </MonTooltipTrigger>
    );
  }

  if (Array.isArray(ctxVal)) {
    return (
      <Fragment key={key}>
        {ctxVal.map((item, idx) => (
          <Fragment key={idx}>
            {renderTokenValue(item, battleState, `${key}-${idx}`, rules)}
            {idx < ctxVal.length - 1 ? ", " : ""}
          </Fragment>
        ))}
      </Fragment>
    );
  }

  return <Fragment key={key}>{formatContextValue(ctxVal)}</Fragment>;
}

export default function LogPanel({
  visibleLogs,
  uiLogs,
  engineLogs = [],
  battleState,
  rules,
  playerData,
  allyPlayerData,
  localPlayerId,
}: LogPanelProps) {
  const [mode, setMode] = useState<"text" | "players" | "engine">("text");
  const [isCollapsed, setIsCollapsed] = useState(false);
  const scrollRef = useRef<HTMLDivElement | null>(null);

  // Automatically scroll to bottom on new logs when in text mode
  useEffect(() => {
    if (scrollRef.current && mode === "text") {
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
          <h3>{mode === "players" ? "Players" : mode === "engine" ? "Engine" : "Logs"}</h3>
        </div>
        <Tabs
          active={mode}
          onChange={setMode}
          options={[
            { value: "text", label: "Text" },
            { value: "players", label: "Players" },
            { value: "engine", label: "Engine" },
          ]}
        />
      </header>

      <div className={styles.scrollArea} ref={scrollRef}>
        {mode === "players" && (
          <PlayerStateViewer
            battleState={battleState}
            playerData={playerData}
            allyPlayerData={allyPlayerData}
            localPlayerId={localPlayerId}
            rules={rules}
          />
        )}

        {mode === "engine" && <EngineLogViewer engineLogs={engineLogs} />}

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

              if (item.kind === "divider") {
                return renderLogDivider(visibleLogs, index, item.subtype);
              }

              if (item.kind === "notice") {
                const noticeType = item.notice.type.toLowerCase();
                const noticeClass = `${styles.noticeLine} ${styles[`${noticeType}Notice`] || ""}`;
                const noticeText = formatNoticeText(item.notice);

                return (
                  <div key={index} className={noticeClass}>
                    <span className={styles.text}>
                      {item.notice.monRef ? (
                        <MonTooltipTrigger
                          monRef={item.notice.monRef}
                          battleState={battleState}
                          rules={rules}
                          preferredPlacement="left"
                        >
                          <span className={styles.monHoverTrigger}>{noticeText}</span>
                        </MonTooltipTrigger>
                      ) : (
                        noticeText
                      )}
                    </span>
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
                      if (ctxVal === undefined) return <Fragment key={i}>{`{{${token.value}}}`}</Fragment>;
                      return renderTokenValue(ctxVal, battleState, i, rules);
                    })}
                  </span>
                </div>
              );
            })}
            {visibleLogs.length === 0 && <p className="empty-state">None</p>}
          </div>
        )}
      </div>
    </div>
  );
}

