import { useState, useMemo } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { formatUiLogEntry } from "../../utils/logFormatter";
import { setBattleError } from "../../store/battlesSlice";
import ErrorBanner from "../Common/ErrorBanner";
import Field from "./Field";
import ActionPanel from "./ActionPanel";
import LogPanel from "./LogPanel";
import BattlePreparationPanel from "./BattlePreparationPanel";
import BattleProposalView from "./BattleProposalView";
import Tabs from "../Common/Tabs";

import styles from "./BattleScreen.module.scss";

export default function BattleScreen() {
  const dispatch = useAppDispatch();
  const [showDebug, setShowDebug] = useState(false);
  const [debugTab, setDebugTab] = useState<"state" | "request" | "player">("state");
  
  const battleId = useAppSelector((state) => state.battles.activeBattleId);
  const connection = useAppSelector((state) => state.connection);
  const battleSession = useAppSelector(
    (state) => (battleId ? state.battles.battles[battleId] : null)
  );

  const activeProposal = useAppSelector(
    (state) => (battleId ? state.proposals.proposals[battleId] : null)
  );

  const visibleLogs = useMemo(() => {
    if (!battleSession || !battleSession.battleState) return [];
    return battleSession.uiLogs
      .map((e) => formatUiLogEntry(e, battleSession.battleState!))
      .filter((formatted): formatted is string => formatted !== null);
  }, [battleSession]);

  // If activeProposal exists, but actual battleSession does not, render the proposal wait state
  if (battleId && activeProposal && !battleSession) {
    return (
      <BattleProposalView
        battleId={battleId}
        activeProposal={activeProposal}
        connection={connection}
      />
    );
  }

  if (!battleId || !battleSession) {
    return (
      <div className={styles.placeholder}>
        <p>Select or join a battle session from the sidebar to play.</p>
      </div>
    );
  }

  const isReconnecting = connection.status === "connecting";
  const isPreparing = battleSession.serviceBattle?.state === "preparing" ||
                      battleSession.battleState?.phase === "pre_battle";

  const opposingSide = battleSession.battleState?.field?.sides?.find(
    (side) => side.name !== connection.playerId
  );
  const opponentName = opposingSide?.name || "Opponent";

  return (
    <div className="page-container">
      {/* Network Lost Overlays */}
      {isReconnecting && (
        <div className={styles.modalOverlay}>
          <div className={styles.modalCard}>
            <div className="spinner" />
            <h3>Connection Lost</h3>
            <p>Re-establishing connection...</p>
          </div>
        </div>
      )}

      <header className={`${styles.screenHeader} flex-row justify-between align-center flex-tablet-col gap-l`}>
        <div className={`${styles.titleInfo} flex-col gap-xs`}>
          <h2>Battle</h2>
          <span className={styles.battleId}>ID: {battleId}</span>
        </div>
        <div className={`${styles.headerControls} flex-row align-center gap-m`}>
          <div className={styles.devToolsPanel}>
            <button
              className={`${styles.devBtn} ${showDebug ? styles.devBtnActive : ""}`}
              onClick={() => setShowDebug(!showDebug)}
              title="Toggle Debug JSON View"
            >
              Debug JSON {showDebug ? "ON" : "OFF"}
            </button>
          </div>
          <div className={styles.vsBadge}>
            @{connection.playerId} <span className={styles.vsText}>VS</span> @{opponentName}
          </div>
        </div>
      </header>

      {battleSession.error && !isPreparing && (
        <ErrorBanner 
          message={battleSession.error} 
          onClear={() => dispatch(setBattleError({ battleId, error: null }))}
        />
      )}

      {showDebug ? (
        <div className={`card ${styles.debugContainer} flex-col gap-m`}>
          <Tabs
            active={debugTab}
            onChange={setDebugTab}
            options={[
              { value: "state", label: "Battle State" },
              { value: "request", label: "Active Request" },
              { value: "player", label: "Player Data" },
            ]}
          />
          <div className={styles.debugJsonContainer}>
            {debugTab === "state" && (
              <>
                <h4>BattleState</h4>
                <pre className={styles.debugJson}>
                  {JSON.stringify(battleSession.battleState, null, 2)}
                </pre>
              </>
            )}
            {debugTab === "request" && (
              <>
                <h4>Request</h4>
                <pre className={styles.debugJson}>
                  {JSON.stringify(battleSession.activeRequest, null, 2)}
                </pre>
              </>
            )}
            {debugTab === "player" && (
              <>
                <h4>PlayerData</h4>
                <pre className={styles.debugJson}>
                  {JSON.stringify(battleSession.playerData, null, 2)}
                </pre>
              </>
            )}
          </div>
        </div>
      ) : isPreparing ? (
        <div className={styles.workspaceGrid}>
          {/* Left Column: Team selection panel */}
          <section className={`${styles.leftColumn} flex-col gap-m`}>
            <BattlePreparationPanel battleId={battleId} />
          </section>

          {/* Right Column: Log panel only */}
          <section className={`${styles.rightColumn} flex-col gap-s`}>
            <LogPanel
              visibleLogs={visibleLogs}
              uiLogs={battleSession.uiLogs}
              engineLogs={battleSession.engineLogs}
            />
          </section>
        </div>
      ) : (
        <div className={styles.workspaceGrid}>
          {/* Left Arena half */}
          <section className={`${styles.leftColumn} flex-col gap-m`}>
            <Field battleState={battleSession.battleState} />
            <ActionPanel
              battleId={battleId}
              request={battleSession.activeRequest}
              playerData={battleSession.playerData}
              playbackPending={false}
              isLoading={battleSession.isLoading}
              errorMessage={battleSession.error}
            />
          </section>

          {/* Right Dashboard column */}
          <section className={`${styles.rightColumn} flex-col gap-s`}>
            <LogPanel
              visibleLogs={visibleLogs}
              uiLogs={battleSession.uiLogs}
              engineLogs={battleSession.engineLogs}
            />
          </section>
        </div>
      )}
    </div>
  );
}
