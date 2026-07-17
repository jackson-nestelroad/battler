import { useEffect, useState } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { submitBattleTeam } from "../../core/wamp";
import { removeProposal } from "../../store/proposalsSlice";
import { switchActiveBattle } from "../../store/battlesSlice";
import ErrorBanner from "../Common/ErrorBanner";
import BattleSidesList from "../Common/BattleSidesList";
import styles from "./BattlePreparationPanel.module.scss";

interface BattlePreparationPanelProps {
  battleId: string;
}

export default function BattlePreparationPanel({ battleId }: BattlePreparationPanelProps) {
  const dispatch = useAppDispatch();

  const battleSession = useAppSelector((state) => state.battles.battles[battleId]);
  const activeProposal = useAppSelector((state) => state.proposals.proposals[battleId]);
  const teams = useAppSelector((state) => state.teams.teams);
  const defaultTeam = useAppSelector((state) => state.teams.defaultTeam);
  const teamOrder = useAppSelector((state) => state.teams.teamOrder);

  const teamNames =
    teamOrder.length > 0 ? teamOrder.filter((name) => teams[name]) : Object.keys(teams);

  const [selectedTeam, setSelectedTeam] = useState("");
  const [timeLeft, setTimeLeft] = useState<number | null>(null);

  // Sync selectedTeam to default on load
  useEffect(() => {
    if (defaultTeam && teams[defaultTeam]) {
      setSelectedTeam(defaultTeam);
    } else if (teamNames.length > 0) {
      setSelectedTeam(teamNames[0]);
    }
  }, [defaultTeam, teams, teamNames]);

  // Countdown timer logic
  useEffect(() => {
    if (!activeProposal) return;
    const deadlineSecs = activeProposal.deadline.secs_since_epoch;

    const updateTimer = () => {
      const now = Math.floor(Date.now() / 1000);
      const diff = deadlineSecs - now;
      setTimeLeft(Math.max(0, diff));
    };

    updateTimer();
    const interval = setInterval(updateTimer, 1000);
    return () => clearInterval(interval);
  }, [activeProposal]);

  const handleSubmitTeam = () => {
    if (!selectedTeam) return;
    const teamMembers = teams[selectedTeam] || [];
    dispatch(submitBattleTeam({ battleId, team: teamMembers }));
  };

  const handleDismiss = () => {
    dispatch(removeProposal(battleId));
    dispatch(switchActiveBattle(null));
  };

  const isExpired = timeLeft === 0 || activeProposal?.deletionReason;

  if (isExpired) {
    const reason = activeProposal?.deletionReason || "Deadline exceeded";
    return (
      <div className={styles.container}>
        <div className={`card ${styles.expiredCard}`}>
          <h3>Proposal Failed</h3>
          <p className="alert alert-danger">Reason: {reason}</p>
          <button onClick={handleDismiss} className="btn btn-primary">
            Return to Lobby
          </button>
        </div>
      </div>
    );
  }

  const sides = battleSession?.serviceBattle?.sides || [];

  return (
    <div className={styles.container}>
      <div className="card">
        <div className={styles.cardHeader}>
          <h3>Battle Preparation</h3>
          <p className={styles.subtitle}>Select your team.</p>
          {timeLeft !== null && (
            <div className={`badge ${timeLeft < 15 ? "badge-danger" : "badge-warning"}`}>
              Time Remaining: {timeLeft}s
            </div>
          )}
        </div>

        {/* Player readiness checklist */}
        <div className={styles.readinessSection}>
          <h4>Player Readiness Status</h4>
          <BattleSidesList sides={sides} isProposal={false} />
        </div>

        {/* Team Selection Section */}
        <div className={styles.teamSelectionSection}>
          <label htmlFor="battle-team-select">Choose Your Battle Team:</label>
          {teamNames.length > 0 ? (
            <div className={styles.selectionRow}>
              <select
                id="battle-team-select"
                value={selectedTeam}
                onChange={(e) => setSelectedTeam(e.target.value)}
                disabled={battleSession?.isLoading}
              >
                {teamNames.map((name) => (
                  <option key={name} value={name}>
                    {name} ({teams[name].length} Pokémon)
                  </option>
                ))}
              </select>
              <button
                onClick={handleSubmitTeam}
                className="btn btn-success"
                disabled={!selectedTeam || battleSession?.isLoading}
              >
                {battleSession?.isLoading ? "Confirming..." : "Confirm Team"}
              </button>
            </div>
          ) : (
            <p className="alert alert-warning">
              You have no teams! Please go to the <strong>Teams Editor</strong> first.
            </p>
          )}
        </div>

        {/* Validation / error reporting */}
        {battleSession?.error && (
          <div className={styles.errorSection}>
            <ErrorBanner message={battleSession.error} />
          </div>
        )}
      </div>
    </div>
  );
}
