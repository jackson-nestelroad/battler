import { useMemo, useState } from "react";
import { classifyEngineLog } from "../../utils/engineLogClassifier";
import styles from "./EngineLogViewer.module.scss";

interface EngineLogViewerProps {
  engineLogs: string[];
  showControls?: boolean;
}

export default function EngineLogViewer({
  engineLogs,
  showControls = true,
}: EngineLogViewerProps) {
  const [hideTimers, setHideTimers] = useState(false);

  const displayLogs = useMemo(() => {
    if (!hideTimers) {
      return engineLogs;
    }
    return engineLogs.filter((log) => !log.startsWith("-battlerservice:timer|"));
  }, [engineLogs, hideTimers]);

  return (
    <div className="flex-col gap-xs">
      {showControls && (
        <div className="flex-row align-center justify-end border-bottom pb-xs mb-xs">
          <label className={styles.filterToggle}>
            <input
              type="checkbox"
              checked={hideTimers}
              onChange={(e) => setHideTimers(e.target.checked)}
            />
            <span>Hide timers</span>
          </label>
        </div>
      )}
      <div className="flex-col gap-xxs">
        {displayLogs.map((log, index) => {
          const category = classifyEngineLog(log);
          return (
            <div key={index} className={`${styles.logLine} ${styles[category]}`}>
              <span className={styles.indicator}>#</span>
              <span className={styles.text}>{log}</span>
            </div>
          );
        })}
        {displayLogs.length === 0 && <p className={styles.emptyLogs}>None</p>}
      </div>
    </div>
  );
}
