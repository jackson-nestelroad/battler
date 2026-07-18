import { useEffect, useState, useMemo } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { submitBattleTeam } from "../../core/wamp";
import ErrorBanner from "../Common/ErrorBanner";
import BattleSidesList from "../Common/BattleSidesList";
import CountdownTimer from "../Common/CountdownTimer";
import styles from "./BattlePreparationPanel.module.scss";

interface BattlePreparationPanelProps {
  battleId: string;
}

export default function BattlePreparationPanel({ battleId }: BattlePreparationPanelProps) {
  const dispatch = useAppDispatch();

  const battleSession = useAppSelector((state) => state.battles.battles[battleId]);
  const activeProposal = useAppSelector((state) => {
    if (!battleId) return null;
    return (
      state.proposals.proposals[battleId] ||
      Object.values(state.proposals.proposals).find((p) => p.battle === battleId) ||
      null
    );
  });
  const teams = useAppSelector((state) => state.teams.teams);
  const defaultTeam = useAppSelector((state) => state.teams.defaultTeam);
  const teamOrder = useAppSelector((state) => state.teams.teamOrder);

  const teamNames = useMemo(() => {
    return teamOrder.length > 0 ? teamOrder.filter((name) => teams[name]) : Object.keys(teams);
  }, [teamOrder, teams]);

  const [selectedTeam, setSelectedTeam] = useState("");

  // Sync selectedTeam to default on load or if the currently selected team is no longer available/empty
  useEffect(() => {
    if (!selectedTeam || !teams[selectedTeam]) {
      if (defaultTeam && teams[defaultTeam]) {
        setSelectedTeam(defaultTeam);
      } else if (teamNames.length > 0) {
        setSelectedTeam(teamNames[0]);
      }
    }
  }, [defaultTeam, teams, teamNames, selectedTeam]);

  const handleSubmitTeam = () => {
    if (!selectedTeam) return;
    const teamMembers = teams[selectedTeam] || [];
    dispatch(submitBattleTeam({ battleId, team: teamMembers }));
  };

  const sides = battleSession?.serviceBattle?.sides || [];

  return (
    <div className={styles.container}>
      <div className="card">
        <div className={styles.cardHeader}>
          <h3>Preparation</h3>
          {activeProposal && (
            <CountdownTimer
              deadlineSecs={activeProposal.deadline.secs_since_epoch}
              prefix="Remaining: "
              badgeMode={true}
            />
          )}
        </div>

        {/* Player readiness checklist */}
        <div className="flex-col gap-m">
          <h4 className={styles.sectionHeader}>Players</h4>
          <BattleSidesList sides={sides} isProposal={false} />
        </div>

        {/* Team Selection Section */}
        <div className={styles.teamSelectionSection}>
          <label htmlFor="battle-team-select">Team:</label>
          {teamNames.length > 0 ? (
            <div className="flex-row flex-mobile-col gap-s">
              <select
                id="battle-team-select"
                className="flex-1"
                value={selectedTeam}
                onChange={(e) => setSelectedTeam(e.target.value)}
                disabled={battleSession?.isLoading}
              >
                {teamNames.map((name) => (
                  <option key={name} value={name}>
                    {name} ({teams[name].length})
                  </option>
                ))}
              </select>
              <button
                onClick={handleSubmitTeam}
                className="btn btn-success"
                disabled={!selectedTeam || battleSession?.isLoading}
              >
                {battleSession?.isLoading ? "Confirming..." : "Confirm"}
              </button>
            </div>
          ) : (
            <p className="alert alert-warning">
              No teams configured. Go to <strong>Teams</strong>.
            </p>
          )}
        </div>

        {/* Validation / error reporting */}
        {battleSession?.error && <ErrorBanner message={battleSession.error} />}
      </div>
    </div>
  );
}
