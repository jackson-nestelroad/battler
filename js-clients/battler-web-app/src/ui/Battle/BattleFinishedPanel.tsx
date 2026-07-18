import { useAppSelector } from "../../store/store";
import styles from "./BattleFinishedPanel.module.scss";
import { encodeBase64Utf8 } from "../../utils/replay";

interface BattleFinishedPanelProps {
  battleId: string;
}

export default function BattleFinishedPanel({ battleId }: BattleFinishedPanelProps) {
  const battleSession = useAppSelector((state) => state.battles.battles[battleId]);

  const handleExport = () => {
    if (!battleSession) return;
    const replayData = {
      battleId,
      engineLogs: battleSession.engineLogs,
    };
    const jsonStr = JSON.stringify(replayData);
    const base64Str = encodeBase64Utf8(jsonStr);
    const dataStr = "data:text/plain;charset=utf-8," + encodeURIComponent(base64Str);
    const downloadAnchor = document.createElement("a");
    downloadAnchor.setAttribute("href", dataStr);
    downloadAnchor.setAttribute("download", `replay-${battleId}.battler`);
    document.body.appendChild(downloadAnchor);
    downloadAnchor.click();
    downloadAnchor.remove();
  };

  return (
    <div className={styles.finishedPanel}>
      <h3>Battle Finished</h3>
      <div className="flex-row gap-s align-center justify-center">
        <button className="btn btn-primary" onClick={handleExport}>
          Export Replay
        </button>
      </div>
    </div>
  );
}
