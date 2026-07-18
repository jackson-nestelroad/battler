import { useState, useMemo } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { formatUiLogEntry } from "../../utils/logFormatter";
import { setBattleError, removeBattle } from "../../store/battlesSlice";
import { refreshBattleSession } from "../../core/wamp";
import ErrorBanner from "../Common/ErrorBanner";
import Field from "./Field";
import ActionPanel from "./ActionPanel";
import ReplayPanel from "./ReplayPanel";
import BattleFinishedPanel from "./BattleFinishedPanel";
import LogPanel from "./LogPanel";
import BattlePreparationPanel from "./BattlePreparationPanel";
import BattleProposalView from "./BattleProposalView";
import Tabs from "../Common/Tabs";
import ConnectForm from "../Common/ConnectForm";
import { getOpponentName } from "../../utils/battle";

import styles from "./BattleScreen.module.scss";

export default function BattleScreen() {
  const dispatch = useAppDispatch();
  const [showDebug, setShowDebug] = useState(false);
  const [debugTab, setDebugTab] = useState<"state" | "request" | "player">("state");

  const battleId = useAppSelector((state) => state.battles.activeBattleId);
  const currentView = useAppSelector((state) => state.battles.currentView);
  const connection = useAppSelector((state) => state.connection);
  const battleSession = useAppSelector((state) =>
    battleId ? state.battles.battles[battleId] : null,
  );

  const [isRefreshing, setIsRefreshing] = useState(false);

  const handleRefresh = async () => {
    if (!battleId || !connection.playerId) return;
    setIsRefreshing(true);
    try {
      await dispatch(refreshBattleSession({ battleId, playerId: connection.playerId })).unwrap();
    } catch (err) {
      console.error(err);
    } finally {
      setIsRefreshing(false);
    }
  };

  const activeProposal = useAppSelector((state) => {
    if (!battleId) return null;
    return (
      state.proposals.proposals[battleId] ||
      Object.values(state.proposals.proposals).find((p) => p.battle === battleId) ||
      null
    );
  });

  const visibleLogs = useMemo(() => {
    if (!battleSession || !battleSession.battleState) return [];
    return battleSession.uiLogs
      .map((e) => formatUiLogEntry(e, battleSession.battleState!))
      .filter((formatted): formatted is string => formatted !== null);
  }, [battleSession]);

  const isReplay = !!battleSession?.isReplay;

  if (
    !isReplay &&
    (connection.status === "disconnected" ||
      (connection.status === "connecting" && !connection.playerId))
  ) {
    return <ConnectForm />;
  }

  if (!battleId) {
    return (
      <div className={styles.placeholder}>
        <p>Select a battle from the sidebar.</p>
      </div>
    );
  }

  if (currentView === "proposal" && activeProposal) {
    return (
      <BattleProposalView
        battleId={battleId}
        activeProposal={activeProposal}
        connection={connection}
      />
    );
  }

  if (!battleSession) {
    if (activeProposal) {
      return (
        <BattleProposalView
          battleId={battleId}
          activeProposal={activeProposal}
          connection={connection}
        />
      );
    }
    return (
      <div className={styles.placeholder}>
        <div className="flex-col align-center gap-m">
          <div className="spinner" />
          <p>Loading...</p>
        </div>
      </div>
    );
  }

  // If battle session is loading, display loading spinner
  if (!battleSession.battleState && battleSession.isLoading) {
    return (
      <div className={styles.placeholder}>
        <div className="flex-col align-center gap-m">
          <div className="spinner" />
          <p>Loading...</p>
        </div>
      </div>
    );
  }

  if (!battleSession.battleState && battleSession.error) {
    const isDeleted = battleSession.isDeleted;
    const isProposalRoute = currentView === "proposal";
    const headerText = isProposalRoute
      ? "Not Found"
      : isDeleted
        ? "Deleted"
        : "Not Found";
    const descText = isProposalRoute
      ? "Proposal no longer active."
      : isDeleted
        ? "Battle no longer exists."
        : "Battle not found.";
    return (
      <div className={styles.placeholder}>
        <div className={`flex-col align-center gap-m text-center ${styles.errorCard}`}>
          <div className="alert alert-danger w-full">
            <div className="flex-col align-start gap-xs text-left">
              <h4>{headerText}</h4>
              <p>{battleSession.error}</p>
            </div>
          </div>
          <p>{descText}</p>
          <button onClick={() => dispatch(removeBattle(battleId))} className="btn btn-primary">
            ← Lobby
          </button>
        </div>
      </div>
    );
  }

  const isReconnecting = connection.status === "connecting";
  const isPreparing =
    battleSession.serviceBattle?.state === "preparing" ||
    battleSession.battleState?.phase === "pre_battle";
  const isFinished = battleSession.battleState?.phase === "finished";

  const side0 =
    battleSession.battleState?.field?.sides?.[0] ||
    battleSession.serviceBattle?.sides?.[0] ||
    activeProposal?.sides?.[0];
  const side1 =
    battleSession.battleState?.field?.sides?.[1] ||
    battleSession.serviceBattle?.sides?.[1] ||
    activeProposal?.sides?.[1];
  const player0Name = side0?.name || "Player 1";
  const player1Name = side1?.name || "Player 2";

  const opponentName = getOpponentName(
    connection.playerId,
    battleSession.battleState,
    battleSession.serviceBattle,
    activeProposal,
  );

  const p0 = isReplay ? player0Name : connection.playerId || player0Name;
  const p1 = isReplay ? player1Name : opponentName;

  return (
    <div className="page-container">
      {/* Network Lost Overlays */}
      {isReconnecting && (
        <div className={styles.modalOverlay}>
          <div className={styles.modalCard}>
            <div className="spinner" />
            <h3>Offline</h3>
            <p>Reconnecting...</p>
          </div>
        </div>
      )}

      <header
        className={`${styles.screenHeader} flex-row justify-between align-center flex-tablet-col gap-l`}
      >
        <div className={`${styles.titleInfo} flex-col gap-xs`}>
          <h2>Battle</h2>
          <span className={styles.battleId}>ID: {battleId}</span>
        </div>
        <div className={`${styles.headerControls} flex-row align-center gap-m`}>
          <button
            onClick={handleRefresh}
            className="btn btn-secondary flex-row align-center gap-xs btn-sm"
            disabled={battleSession?.isLoading || isRefreshing}
            title="Refresh Battle State"
          >
            <span className={isRefreshing ? "spin-icon" : ""}>↻</span> Refresh
          </button>
          <div className={styles.devToolsPanel}>
            <button
              className={`${styles.devBtn} ${showDebug ? styles.devBtnActive : ""}`}
              onClick={() => setShowDebug(!showDebug)}
              title="Toggle Debug JSON View"
            >
              Debug
            </button>
          </div>
          <div className={styles.vsBadge}>
            @{p0} <span className={styles.vsText}>VS</span> @{p1}
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
              { value: "state", label: "State" },
              { value: "request", label: "Request" },
              { value: "player", label: "Player" },
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
            {isReplay ? (
              <div className="card">
                <ReplayPanel battleId={battleId} />
              </div>
            ) : isFinished ? (
              <div className="card">
                <BattleFinishedPanel battleId={battleId} />
              </div>
            ) : (
              <ActionPanel
                battleId={battleId}
                request={battleSession.activeRequest}
                playerData={battleSession.playerData}
                playbackPending={false}
                isLoading={battleSession.isLoading}
                errorMessage={battleSession.choiceError}
              />
            )}
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
