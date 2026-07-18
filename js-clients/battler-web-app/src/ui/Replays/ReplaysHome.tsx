import React, { useState } from "react";
import { useAppDispatch } from "../../store/store";
import { battleReplayLoaded } from "../../store/battlesSlice";
import { precomputeReplayKeyframes, decodeBase64Utf8 } from "../../utils/replay";
import type { SavedReplay } from "../../utils/replay";
import styles from "./ReplaysHome.module.scss";

export default function ReplaysHome() {
  const dispatch = useAppDispatch();
  const [replayError, setReplayError] = useState<string | null>(null);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (event) => {
      let json: SavedReplay;
      try {
        const result = event.target?.result;
        if (typeof result !== "string") {
          setReplayError("Failed to read replay file content.");
          return;
        }
        const base64Str = result.trim();
        const jsonStr = decodeBase64Utf8(base64Str);
        const parsed = JSON.parse(jsonStr);
        if (parsed && typeof parsed.battleId === "string" && Array.isArray(parsed.engineLogs)) {
          json = parsed;
        } else {
          setReplayError("Invalid replay file format. Must contain battleId and engineLogs.");
          return;
        }
      } catch {
        setReplayError("Failed to parse replay file. Make sure it is a valid .battler file.");
        return;
      }

      const { keyframes, maxTurn } = precomputeReplayKeyframes(json.engineLogs);
      if (keyframes.length === 0) {
        setReplayError("Failed to precompute start keyframe from the logs.");
        return;
      }

      dispatch(
        battleReplayLoaded({
          battleId: json.battleId,
          engineLogs: json.engineLogs,
          keyframes,
          maxTurn,
        }),
      );
      setReplayError(null);
    };
    reader.readAsText(file);
  };

  return (
    <div className="page-container scroll-y">
      <header className="dashboard-header">
        <h1>Replays</h1>
      </header>

      <div className={styles.replaysHome}>
        <section className="card">
          <div className="card-header">
            <h3>Load Replay</h3>
          </div>
          <div className="flex-col gap-s w-full">
            <p className={styles.replayHelpText}>Upload a .battler replay file.</p>
            <div className="flex-row align-center gap-m">
              <label className={`${styles.fileInputLabel} btn btn-secondary`}>
                Choose File
                <input type="file" accept=".battler" onChange={handleFileChange} />
              </label>
              {replayError && <div className={styles.replayErrorText}>{replayError}</div>}
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
