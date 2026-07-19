import { useState, useEffect, useMemo } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { formatUiLogEntry } from "../../utils/logFormatter";
import { setBattleError } from "../../store/battlesSlice";
import { refreshBattleSession, closeBattleSession } from "../../core/wamp";
import ErrorBanner from "../Common/ErrorBanner";
import Field from "./Field";
import ActionPanel from "./ActionPanel";
import ReplayPanel from "./ReplayPanel";
import BattleFinishedPanel from "./BattleFinishedPanel";
import LogPanel from "./LogPanel";
import BattlePreparationPanel from "./BattlePreparationPanel";
import BattleProposalView from "./BattleProposalView";
import Tabs from "../Common/Tabs";
import CopyableId from "../Common/CopyableId";
import RefreshButton from "../Common/RefreshButton";
import { getBattleTitle } from "../../utils/battle";

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

  const title = useMemo(() => {
    if (!battleSession) return "";
    return getBattleTitle(
      battleSession.battleState,
      battleSession.serviceBattle,
      battleSession.isProposal ? activeProposal : null,
    );
  }, [battleSession, activeProposal]);

  useEffect(() => {
    if (title) {
      document.title = `${title} | Battler`;
    }
    return () => {
      document.title = "Battler";
    };
  }, [title]);

  const visibleLogs = useMemo(() => {
    if (!battleSession || !battleSession.battleState) return [];
    return battleSession.uiLogs
      .map((e) => formatUiLogEntry(e, battleSession.battleState!))
      .filter((formatted): formatted is string => formatted !== null);
  }, [battleSession]);

  const isReplay = !!battleSession?.isReplay;



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
    const headerText = isProposalRoute ? "Not found" : isDeleted ? "Deleted" : "Not found";
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
          <button onClick={() => dispatch(closeBattleSession(battleId))} className="btn btn-primary">
            ← Lobby
          </button>
        </div>
      </div>
    );
  }

  const isPreparing =
    battleSession.serviceBattle?.state === "preparing" ||
    battleSession.battleState?.phase === "pre_battle";
  const isFinished = battleSession.battleState?.phase === "finished";

  return (
    <div className="page-container">

      <header className={`${styles.screenHeader} flex-row justify-between align-center gap-m`}>
        <div className={`${styles.titleInfo} flex-col gap-xs`}>
          <h2>{title}</h2>
          <span className={styles.battleId}>
            <span className={styles.battleFormat}>{isReplay ? "Replay" : "Battle"}</span> •{" "}
            <CopyableId id={battleId} type={isReplay ? "replay" : "battle"} />
          </span>
        </div>
        <div className={`${styles.headerControls} flex-row align-center`}>
          {!isReplay && (
            <RefreshButton
              onClick={handleRefresh}
              isRefreshing={battleSession?.isLoading || isRefreshing}
            />
          )}
          <button
            className={`btn btn-sm ${showDebug ? "btn-primary" : "btn-secondary"}`}
            onClick={() => setShowDebug(!showDebug)}
            title="Toggle Debug JSON View"
          >
            <span className="btn-icon-mobile">🐞</span>
            <span className="btn-text-desktop">Debug</span>
          </button>
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
