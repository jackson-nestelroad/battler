import { useAppSelector } from "../../store/store";
import { downloadReplayFile } from "../../utils/replay";
import styles from "./BattleFinishedPanel.module.scss";

interface BattleFinishedPanelProps {
  battleId: string;
}

export default function BattleFinishedPanel({ battleId }: BattleFinishedPanelProps) {
  const battleSession = useAppSelector((state) => state.battles.battles[battleId]);

  const handleExport = () => {
    if (!battleSession) return;
    const engineLogs = battleSession.isReplay
      ? battleSession.replayEngineLogs
      : battleSession.engineLogs;
    downloadReplayFile({
      battleId,
      engineLogs,
      metadata: battleSession.serviceBattle?.metadata || battleSession.metadata,
    });
  };

  return (
    <div className={styles.finishedPanel}>
      <h3>{battleSession?.isDeleted ? "Deleted" : "Finished"}</h3>
      <div className="flex-row gap-s align-center justify-center">
        <button className="btn btn-primary" onClick={handleExport}>
          Export
        </button>
      </div>
    </div>
  );
}
