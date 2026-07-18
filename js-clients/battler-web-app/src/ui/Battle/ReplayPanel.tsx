import React, { useState, useEffect } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { setReplayTurn, isReplaySession } from "../../store/battlesSlice";
import styles from "./ReplayPanel.module.scss";

interface ReplayPanelProps {
  battleId: string;
}

export default function ReplayPanel({ battleId }: ReplayPanelProps) {
  const dispatch = useAppDispatch();
  const battleSession = useAppSelector((state) => state.battles.battles[battleId]);
  const replaySession = isReplaySession(battleSession) ? battleSession : null;

  const turnNumber = replaySession?.replayCurrentTurn || 0;
  const maxTurn = (replaySession?.replayStates.length || 1) - 1;
  // Replay play state
  const [isPlaying, setIsPlaying] = useState(false);

  useEffect(() => {
    let intervalId: ReturnType<typeof setInterval> | null = null;
    if (isPlaying && replaySession) {
      intervalId = setInterval(() => {
        if (turnNumber < maxTurn) {
          dispatch(setReplayTurn({ battleId, turn: turnNumber + 1 }));
        } else {
          setIsPlaying(false);
        }
      }, 1500);
    }
    return () => {
      if (intervalId) clearInterval(intervalId);
    };
  }, [isPlaying, turnNumber, maxTurn, battleId, dispatch, replaySession]);

  const handleStep = (targetTurn: number) => {
    if (targetTurn < 0 || targetTurn > maxTurn) return;
    dispatch(setReplayTurn({ battleId, turn: targetTurn }));
  };

  return (
    <div className={styles.replayPanel}>
      <h3>Replay</h3>

      <div className={styles.turnScrubber}>
        <div className={`${styles.scrubberHeader} flex-row justify-between w-full`}>
          <span>
            {turnNumber === 0 ? "Start" : turnNumber === maxTurn ? "End" : `Turn ${turnNumber}`}
          </span>
          <span className={styles.replaySubtitle}>End (Turn {maxTurn - 1})</span>
        </div>
        <input
          type="range"
          min={0}
          max={maxTurn}
          value={turnNumber}
          onChange={(e) => handleStep(parseInt(e.target.value, 10))}
          className={styles.scrubberRange}
          style={
            {
              "--range-progress": `${maxTurn > 0 ? (turnNumber / maxTurn) * 100 : 0}%`,
            } as React.CSSProperties
          }
        />
      </div>

      <div className="flex-row justify-center gap-s flex-wrap w-full">
        <button
          className="btn btn-secondary btn-sm"
          onClick={() => handleStep(0)}
          disabled={turnNumber === 0}
          title="Go to Start"
        >
          ⏮ First
        </button>
        <button
          className="btn btn-secondary btn-sm"
          onClick={() => handleStep(turnNumber - 1)}
          disabled={turnNumber === 0}
          title="Previous Turn"
        >
          ◀ Prev
        </button>
        <button
          className={`btn ${isPlaying ? "btn-warning" : "btn-primary"} btn-sm`}
          onClick={() => setIsPlaying(!isPlaying)}
          disabled={turnNumber === maxTurn && !isPlaying}
          title={isPlaying ? "Pause Playback" : "Play Playback"}
        >
          {isPlaying ? "⏸ Pause" : "▶ Play"}
        </button>
        <button
          className="btn btn-secondary btn-sm"
          onClick={() => handleStep(turnNumber + 1)}
          disabled={turnNumber === maxTurn}
          title="Next Turn"
        >
          Next ▶
        </button>
        <button
          className="btn btn-secondary btn-sm"
          onClick={() => handleStep(maxTurn)}
          disabled={turnNumber === maxTurn}
          title="Go to End"
        >
          Last ⏭
        </button>
      </div>
    </div>
  );
}
