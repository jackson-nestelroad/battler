import { useState } from "react";
import type { BattleType, MonData } from "battler-types";
import type { FormPlayer } from "./proposalTypes";
import TeamSelect from "../Common/TeamSelect";
import styles from "./ProposalForm.module.scss";

interface PlayerSlotCardProps {
  side: 1 | 2;
  index: number;
  player: FormPlayer;
  isProposer: boolean;
  format: BattleType;
  teams: Record<string, MonData[]>;
  teamNames: string[];
  defaultTeam: string | null;
  onRemove: () => void;
  onChange: (fields: Partial<FormPlayer>) => void;
}

export default function PlayerSlotCard({
  side,
  index,
  player,
  isProposer,
  format,
  teams,
  teamNames,
  defaultTeam,
  onRemove,
  onChange,
}: PlayerSlotCardProps) {
  const [isOpen, setIsOpen] = useState(false);
  return (
    <div className={styles.playerSlotCard}>
      <div className="flex-row justify-between align-center mb-xs">
        <span className={styles.playerSlotLabel}>
          {isProposer ? "You" : `Player #${index + 1}`}
        </span>
        {!isProposer && format === "Multi" && (
          <button type="button" className="btn btn-danger btn-sm" onClick={onRemove}>
            Remove
          </button>
        )}
      </div>

      <div className="flex-col gap-s">
        <div className={styles.playerInputsGrid}>
          <div className="form-group flex-1">
            <label>Control</label>
            <select
              value={player.controlType}
              onChange={(e) => {
                const val = e.target.value as "human" | "ai";
                onChange({
                  controlType: val,
                  id: val === "ai" ? "" : player.id,
                  selectedTeam: val === "ai" ? defaultTeam || teamNames[0] || "" : undefined,
                });
              }}
              disabled={isProposer}
            >
              <option value="human">Player</option>
              <option value="ai">AI</option>
            </select>
          </div>

          {player.controlType === "human" ? (
            <div className="form-group flex-1">
              <label>Player name</label>
              <input
                type="text"
                value={player.id}
                onChange={(e) => onChange({ id: e.target.value })}
                placeholder="Player name"
                disabled={isProposer}
                required
              />
            </div>
          ) : (
            <div className="form-group flex-1">
              <label>AI type</label>
              <select value="random" disabled>
                <option value="random">Random AI</option>
              </select>
            </div>
          )}

          <div className="form-group flex-1">
            <label>Player type</label>
            <select
              value={player.playerType}
              onChange={(e) =>
                onChange({
                  playerType: e.target.value as "trainer" | "wild" | "protagonist",
                })
              }
            >
              <option value="trainer">Trainer</option>
              <option value="wild">Wild</option>
              <option value="protagonist">Protagonist</option>
            </select>
          </div>

          {player.controlType === "ai" && (
            <div className="form-group flex-1">
              <label htmlFor={`ai-team-${side}-${index}`}>AI team</label>
              {teamNames.length > 0 ? (
                <TeamSelect
                  id={`ai-team-${side}-${index}`}
                  value={player.selectedTeam || ""}
                  onChange={(val) => onChange({ selectedTeam: val })}
                  teamNames={teamNames}
                  teams={teams}
                  required
                />
              ) : (
                <span className="text-danger text-sm">No teams. Go to Teams.</span>
              )}
            </div>
          )}
        </div>

        <div>
          <button
            type="button"
            className={`btn btn-secondary btn-sm ${styles.advancedSettingsToggle}`}
            onClick={() => setIsOpen(!isOpen)}
          >
            {isOpen ? "Hide options" : "Player options"}
          </button>
        </div>

        {isOpen && (
          <div className={styles.advancedPlayerSettings}>
            <div className={styles.checkboxGrid}>
              <label className={styles.checkboxLabel}>
                <input
                  type="checkbox"
                  checked={player.hasAffection}
                  onChange={(e) => onChange({ hasAffection: e.target.checked })}
                />
                <span>Affection</span>
              </label>

              <label className={styles.checkboxLabel}>
                <input
                  type="checkbox"
                  checked={player.hasStrictBag}
                  onChange={(e) => onChange({ hasStrictBag: e.target.checked })}
                />
                <span>Strict bag</span>
              </label>

              <label className={styles.checkboxLabel}>
                <input
                  type="checkbox"
                  checked={!player.cannotMegaEvolve}
                  onChange={(e) => onChange({ cannotMegaEvolve: !e.target.checked })}
                />
                <span>Mega Evolution</span>
              </label>

              <label className={styles.checkboxLabel}>
                <input
                  type="checkbox"
                  checked={!player.cannotZMove}
                  onChange={(e) => onChange({ cannotZMove: !e.target.checked })}
                />
                <span>Z-Moves</span>
              </label>

              <label className={styles.checkboxLabel}>
                <input
                  type="checkbox"
                  checked={!player.cannotDynamax}
                  onChange={(e) => onChange({ cannotDynamax: !e.target.checked })}
                />
                <span>Dynamax</span>
              </label>

              <label className={styles.checkboxLabel}>
                <input
                  type="checkbox"
                  checked={!player.cannotTerastallize}
                  onChange={(e) => onChange({ cannotTerastallize: !e.target.checked })}
                />
                <span>Terastallization</span>
              </label>
            </div>

            <div className="flex-row gap-s align-center flex-mobile-col">
              <div className="form-group flex-1">
                <label>Mons caught</label>
                <input
                  type="number"
                  min="0"
                  value={player.monsCaught}
                  onChange={(e) => onChange({ monsCaught: Number(e.target.value) })}
                />
              </div>
              {player.playerType === "wild" && (
                <div className="form-group flex-1">
                  <label>Encounter type</label>
                  <select
                    value={player.wildEncounterType}
                    onChange={(e) =>
                      onChange({
                        wildEncounterType: e.target.value as "Normal" | "Fishing",
                      })
                    }
                  >
                    <option value="Normal">Normal</option>
                    <option value="Fishing">Fishing</option>
                  </select>
                </div>
              )}
            </div>

            {player.playerType === "wild" && (
              <div className={styles.checkboxGrid}>
                <label className={styles.checkboxLabel}>
                  <input
                    type="checkbox"
                    checked={player.wildCatchable}
                    onChange={(e) => onChange({ wildCatchable: e.target.checked })}
                  />
                  <span>Catchable</span>
                </label>

                <label className={styles.checkboxLabel}>
                  <input
                    type="checkbox"
                    checked={player.wildEscapable}
                    onChange={(e) => onChange({ wildEscapable: e.target.checked })}
                  />
                  <span>Escapable</span>
                </label>

                <label className={styles.checkboxLabel}>
                  <input
                    type="checkbox"
                    checked={player.wildCanEscape}
                    onChange={(e) => onChange({ wildCanEscape: e.target.checked })}
                  />
                  <span>Can escape</span>
                </label>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
