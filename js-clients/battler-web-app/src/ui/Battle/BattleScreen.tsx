import { useEffect, useMemo, useState } from "react";
import { checkBattleStatus, closeBattleSession, refreshBattleSession } from "../../core/wamp";
import { isSpectatorSession, selectBattle, setBattleError } from "../../store/battlesSlice";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { getBattleTitle } from "../../utils/battle";
import { formatUiLogEntry } from "../../utils/logFormatter";
import BattleDetailsGrid from "../Common/BattleDetailsGrid";
import CopyableId from "../Common/CopyableId";
import ErrorBanner from "../Common/ErrorBanner";
import RefreshButton from "../Common/RefreshButton";
import Tabs from "../Common/Tabs";
import ActionPanel from "./ActionPanel";
import BattleFinishedPanel from "./BattleFinishedPanel";
import BattlePreparationPanel from "./BattlePreparationPanel";
import BattleProposalView from "./BattleProposalView";
import styles from "./BattleScreen.module.scss";
import BattleTimers from "./BattleTimers";
import Field from "./Field";
import LogPanel from "./LogPanel";
import ReplayPanel from "./ReplayPanel";

export default function BattleScreen() {
  const dispatch = useAppDispatch();
  const [showDebug, setShowDebug] = useState(false);
  const [debugTab, setDebugTab] = useState<"state" | "ui_log" | "request" | "player" | "metadata">("state");

  const battleId = useAppSelector((state) => state.battles.activeBattleId);
  const currentView = useAppSelector((state) => state.battles.currentView);
  const connection = useAppSelector((state) => state.connection);
  const battleSession = useAppSelector((state) =>
    battleId ? state.battles.battles[battleId] : null,
  );

  const [isRefreshing, setIsRefreshing] = useState(false);
  const [showDetails, setShowDetails] = useState(false);

  const isFinished = battleSession?.battleState?.phase === "finished";
  const isReplay = !!battleSession?.isReplay;

  useEffect(() => {
    if (!battleId || !isFinished || battleSession?.isDeleted || isReplay) return;
    dispatch(checkBattleStatus(battleId));
  }, [battleId, isFinished, battleSession?.isDeleted, isReplay, dispatch]);

  useEffect(() => {
    setShowDetails(false);
  }, [battleId]);

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
      battleSession.isDeleted,
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
    const isSpectator = isSpectatorSession(battleSession, connection.playerId);
    return battleSession.uiLogs
      .flatMap((e) =>
        formatUiLogEntry(e, battleSession.battleState!, {
          localPlayerId: connection.playerId || undefined,
          isSpectator,
        }),
      );
  }, [battleSession, connection.playerId]);

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
    if (connection.status === "connected") {
      return (
        <div className={styles.placeholder}>
          <div className={`flex-col align-center gap-m text-center ${styles.errorCard}`}>
            <div className="alert alert-danger w-full">
              <div className="flex-col align-start gap-xs text-left">
                <h4>Not Found</h4>
                <p>Battle or proposal no longer exists.</p>
              </div>
            </div>
            <button
              className="btn btn-primary"
              onClick={() => dispatch(selectBattle({ view: "lobby", battleId: null }))}
            >
              ← Lobby
            </button>
          </div>
        </div>
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

  if (!battleSession.battleState && battleSession.error) {
    const isDeleted = battleSession.isDeleted;
    const isProposalRoute = currentView === "proposal";
    const headerText = isDeleted ? "Deleted" : "Not found";
    const descText = isDeleted
      ? isProposalRoute
        ? "Proposal no longer exists"
        : "Battle no longer exists"
      : isProposalRoute
        ? "Proposal no longer active"
        : "Battle not found";
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
          <button
            onClick={() => dispatch(closeBattleSession(battleId))}
            className="btn btn-primary"
          >
            ← Lobby
          </button>
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

  const isPreparing =
    battleSession.serviceBattle?.state === "preparing" ||
    battleSession.battleState?.phase === "pre_battle";

  const metadata = battleSession?.serviceBattle?.metadata || battleSession?.metadata;

  return (
    <div className="page-container">
      <header className="screen-header flex-row justify-between align-center gap-m">
        <div className="screen-header-title flex-col gap-xs">
          <h2>{title}</h2>
          <span className="screen-header-subtitle">
            <span className="screen-header-format">
              {isReplay ? "Replay" : battleSession.isSpectator ? "Spectating" : "Battle"}
            </span>{" "}
            • <CopyableId id={battleId} type={isReplay ? "replay" : "battle"} />
            {metadata && (
              <>
                {" "}
                • <span className="screen-header-format">{metadata.special || metadata.battle_type}</span>
                {((metadata.rules && metadata.rules.length > 0) || metadata.timers || metadata.special) && (
                  <>
                    {" "}
                    •{" "}
                    <span
                      className={styles.detailsToggle}
                      onClick={() => setShowDetails(!showDetails)}
                    >
                      Details {showDetails ? "▲" : "▼"}
                    </span>
                  </>
                )}
              </>
            )}
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

      {showDetails && metadata && (
        <div className={`${styles.detailsDropdown} flex-col gap-s`}>
          <h4 className="details-header">Battle Details</h4>
          <BattleDetailsGrid
            battleType={metadata.battle_type}
            rules={metadata.rules}
            timers={metadata.timers}
            special={metadata.special}
          />
        </div>
      )}

      {battleSession.isDeleted ? (
        <div className="alert alert-danger w-full flex-row align-center justify-between gap-m">
          <div className="flex-col gap-xxs text-left">
            <strong>Battle Deleted</strong>
            <span>{battleSession.error || "Battle no longer exists"}</span>
          </div>
          <button
            onClick={() => dispatch(closeBattleSession(battleId))}
            className="btn btn-primary btn-sm"
          >
            ← Lobby
          </button>
        </div>
      ) : (
        battleSession.error &&
        !isPreparing && (
          <ErrorBanner
            message={battleSession.error}
            onClear={() => dispatch(setBattleError({ battleId, error: null }))}
          />
        )
      )}

      {showDebug ? (
        <div className={`card ${styles.debugContainer} flex-col gap-m`}>
          <Tabs
            active={debugTab}
            onChange={setDebugTab}
            options={[
              { value: "state", label: "State" },
              { value: "ui_log", label: "UI Log" },
              { value: "request", label: "Request" },
              { value: "player", label: "Player" },
              { value: "metadata", label: "Metadata" },
            ]}
          />
          <div className={styles.debugJsonContainer}>
            {debugTab === "state" && (
              <>
                <h4>BattleState</h4>
                <pre className={styles.debugJson}>
                  {JSON.stringify(
                    battleSession.battleState
                      ? { ...battleSession.battleState, ui_log: undefined }
                      : null,
                    null,
                    2
                  )}
                </pre>
              </>
            )}
            {debugTab === "ui_log" && (
              <>
                <h4>UI Log</h4>
                <pre className={styles.debugJson}>
                  {JSON.stringify(battleSession.battleState?.ui_log, null, 2)}
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
            {debugTab === "metadata" && (
              <>
                <h4>BattleMetadata</h4>
                <pre className={styles.debugJson}>{JSON.stringify(metadata, null, 2)}</pre>
              </>
            )}
          </div>
        </div>
      ) : isPreparing ? (
        <div className={styles.workspaceGrid}>
          {/* Left Column: Team selection panel */}
          <section className={`${styles.leftColumn} flex-col gap-m`}>
            <BattleTimers
              activeTimers={battleSession.activeTimers}
              playerId={connection.playerId || undefined}
              battleState={battleSession.battleState}
              serviceBattle={battleSession.serviceBattle}
              isReplay={isReplay}
            />
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
          {/* Left Arena & Command Deck */}
          <section className={`${styles.leftColumn} flex-col gap-m`}>
            <Field
              battleState={battleSession.battleState}
              activeTimers={battleSession.activeTimers}
              playerId={connection.playerId || undefined}
              serviceBattle={battleSession.serviceBattle}
              isReplay={isReplay}
            />
            {isReplay ? (
              <div className="card">
                <ReplayPanel battleId={battleId} />
              </div>
            ) : isFinished || battleSession.isDeleted ? (
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
