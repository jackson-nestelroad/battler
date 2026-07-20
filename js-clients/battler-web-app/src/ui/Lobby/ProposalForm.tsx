import React, { useState, useEffect, useMemo } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { proposeBattle } from "../../core/wamp";
import { selectBattle } from "../../store/battlesSlice";
import { updateProposal } from "../../store/proposalsSlice";
import type { CoreBattleOptions, BattleType, MonData } from "battler-types";

import { setConnectionError } from "../../store/connectionSlice";

import PlayerSlotCard from "./PlayerSlotCard";
import { createDefaultPlayer } from "./proposalTypes";
import type { FormPlayer } from "./proposalTypes";
import AdvancedRulesSection from "./AdvancedRulesSection";
import type { CustomRulesState } from "./AdvancedRulesSection";
import FieldSettingsSection from "./FieldSettingsSection";
import type { FieldSettingsState } from "./FieldSettingsSection";
import TimerSettingsSection, { TIMER_PRESETS } from "./TimerSettingsSection";
import type { TimerSettingsState } from "./TimerSettingsSection";
import TeamSelect from "../Common/TeamSelect";

import styles from "./ProposalForm.module.scss";

const parseBigIntSafe = (val: string): bigint => {
  const parsed = parseInt(val, 10);
  return isNaN(parsed) ? 0n : BigInt(parsed);
};

export default function ProposalForm() {
  const dispatch = useAppDispatch();
  const connection = useAppSelector((state) => state.connection);
  const teams = useAppSelector((state) => state.teams.teams);
  const teamOrder = useAppSelector((state) => state.teams.teamOrder);
  const defaultTeam = useAppSelector((state) => state.teams.defaultTeam);

  const teamNames = useMemo(() => {
    return teamOrder.length > 0 ? teamOrder.filter((name) => teams[name]) : Object.keys(teams);
  }, [teamOrder, teams]);

  // Proposal form state
  const [format, setFormat] = useState<BattleType>("Singles");
  const [showAdvanced, setShowAdvanced] = useState(false);

  // Side 1 Players (proposer is always side1[0])
  const [side1Players, setSide1Players] = useState<FormPlayer[]>([
    createDefaultPlayer(connection.playerId || ""),
  ]);

  // Side 2 Players (starts with 1 slot)
  const [side2Players, setSide2Players] = useState<FormPlayer[]>([createDefaultPlayer()]);

  // Grouped advanced custom rules state
  const [customRules, setCustomRules] = useState<CustomRulesState>({
    preset: "standard",
    speciesClause: true,
    sleepClause: true,
    itemClause: true,
    nicknameClause: true,
    ohkoClause: true,
    evasionClause: true,
    endlessBattleClause: true,
    megaEvolution: false,
    zMoves: false,
    dynamax: false,
    terastallization: false,
    bagItems: false,
    pickedTeamSizeAuto: true,
    pickedTeamSize: 3,
    adjustLevelDownEnabled: false,
    adjustLevelDown: 50,
    defaultLevel: 100,
    maxLevel: 100,
    rulesList: [],
  });

  // Grouped field settings state
  const [fieldSettings, setFieldSettings] = useState<FieldSettingsState>({
    weather: "None",
    terrain: "None",
    environment: "Normal",
    timeOfDay: "Day",
  });

  // Grouped timer settings state
  const [timerSettings, setTimerSettings] = useState<TimerSettingsState>({
    preset: "standard",
    battleTimer: "",
    playerTimer: "",
    actionTimer: "",
    teamPreviewTimer: "",
    proposalTimeout: 60,
  });

  const isAdvancedView = showAdvanced || format === "Multi";

  // Synchronize player ID when proposer is loaded/logged in
  useEffect(() => {
    if (connection.playerId) {
      setSide1Players((prev) =>
        prev.map((p, idx) => (idx === 0 ? { ...p, id: connection.playerId || "" } : p)),
      );
    }
  }, [connection.playerId]);

  // Enforce player counts based on selected format
  useEffect(() => {
    if (format !== "Multi") {
      setSide1Players((prev) => (prev.length > 1 ? prev.slice(0, 1) : prev));
      setSide2Players((prev) => (prev.length > 1 ? prev.slice(0, 1) : prev));
    } else {
      setSide1Players((prev) => {
        if (prev.length < 2) {
          return [...prev, createDefaultPlayer()];
        }
        return prev;
      });
      setSide2Players((prev) => {
        if (prev.length < 2) {
          return [...prev, createDefaultPlayer()];
        }
        return prev;
      });
    }
  }, [format]);

  const addPlayerSlot = (side: 1 | 2) => {
    const newPlayer = createDefaultPlayer();
    if (side === 1) {
      setSide1Players([...side1Players, newPlayer]);
    } else {
      setSide2Players([...side2Players, newPlayer]);
    }
  };

  const removePlayerSlot = (side: 1 | 2, index: number) => {
    if (side === 1) {
      if (index === 0) return;
      setSide1Players(side1Players.filter((_, i) => i !== index));
    } else {
      setSide2Players(side2Players.filter((_, i) => i !== index));
    }
  };

  const updatePlayerSlot = (side: 1 | 2, index: number, fields: Partial<FormPlayer>) => {
    if (side === 1) {
      setSide1Players(side1Players.map((p, i) => (i === index ? { ...p, ...fields } : p)));
    } else {
      setSide2Players(side2Players.map((p, i) => (i === index ? { ...p, ...fields } : p)));
    }
  };

  const handleSendProposal = (e: React.FormEvent) => {
    e.preventDefault();

    // Assign unique CPU player IDs dynamically
    let cpuIndex = 1;
    const finalSide1 = side1Players.map((p, idx) => {
      if (idx === 0) {
        return { ...p, id: connection.playerId || "player-1" };
      }
      if (p.controlType === "ai") {
        return { ...p, id: `ai-random-${cpuIndex++}` };
      }
      return p;
    });

    const finalSide2 = side2Players.map((p) => {
      if (p.controlType === "ai") {
        return { ...p, id: `ai-random-${cpuIndex++}` };
      }
      return p;
    });

    // Check if any human slot username starts with 'ai-'
    const hasReservedAiPrefix =
      finalSide1.some(
        (p) => p.controlType === "human" && p.id.trim().toLowerCase().startsWith("ai-"),
      ) ||
      finalSide2.some(
        (p) => p.controlType === "human" && p.id.trim().toLowerCase().startsWith("ai-"),
      );

    if (hasReservedAiPrefix) {
      dispatch(setConnectionError("Usernames starting with 'ai-' are reserved.", null));
      return;
    }

    // Check if any human slot username is empty
    const hasEmptyHumanName =
      finalSide1.some((p) => p.controlType === "human" && !p.id.trim()) ||
      finalSide2.some((p) => p.controlType === "human" && !p.id.trim());

    if (hasEmptyHumanName) {
      dispatch(setConnectionError("Please enter all player names/IDs.", null));
      return;
    }

    // Check if any AI player does not have a selected team or if the selected team does not exist
    const hasEmptyAiTeam =
      finalSide1.some(
        (p) => p.controlType === "ai" && (!p.selectedTeam || !teams[p.selectedTeam]),
      ) ||
      finalSide2.some((p) => p.controlType === "ai" && (!p.selectedTeam || !teams[p.selectedTeam]));

    if (hasEmptyAiTeam) {
      dispatch(setConnectionError("Please select a valid team for all AI players.", null));
      return;
    }

    // Build format rules array
    let rulesArray: string[] = [];
    if (customRules.preset === "standard") {
      rulesArray = ["Standard"];
    } else if (customRules.preset === "standarddoubles") {
      rulesArray = ["Standard Doubles"];
    } else if (customRules.preset === "flatrules") {
      rulesArray = ["Flat Rules"];
    } else if (customRules.preset === "custom") {
      if (customRules.sleepClause) rulesArray.push("Sleep Clause");
      if (customRules.speciesClause) rulesArray.push("Species Clause");
      if (customRules.itemClause) rulesArray.push("Item Clause");
      if (customRules.nicknameClause) rulesArray.push("Nickname Clause");
      if (customRules.ohkoClause) rulesArray.push("OHKO Clause");
      if (customRules.evasionClause) {
        rulesArray.push("Evasion Items Clause");
        rulesArray.push("Evasion Moves Clause");
      }
      if (customRules.endlessBattleClause) rulesArray.push("Endless Battle Clause");

      // Custom rule flags (Dynamax, Z-Moves, Mega Evolution, Terastallization)
      if (customRules.megaEvolution) rulesArray.push("Mega Evolution");
      if (customRules.zMoves) rulesArray.push("Z-Moves");
      if (customRules.dynamax) rulesArray.push("Dynamax");
      if (customRules.terastallization) rulesArray.push("Terastallization");
      if (customRules.bagItems) rulesArray.push("Bag Items");

      if (!customRules.pickedTeamSizeAuto) {
        rulesArray.push(`Picked Team Size = ${customRules.pickedTeamSize}`);
      }
      if (customRules.adjustLevelDownEnabled) {
        rulesArray.push(`Adjust Level Down = ${customRules.adjustLevelDown}`);
      }
      if (customRules.defaultLevel) rulesArray.push(`Default Level = ${customRules.defaultLevel}`);
      if (customRules.maxLevel) rulesArray.push(`Max Level = ${customRules.maxLevel}`);
      for (const rule of customRules.rulesList) {
        rulesArray.push(rule);
      }
    }

    // TODO: Re-enable when we support Team Preview.
    rulesArray.push("! Team Preview");

    // Build timers
    let battleTimerVal: { secs: bigint; warnings: bigint[] } | null = null;
    let playerTimerVal: { secs: bigint; warnings: bigint[] } | null = null;
    let actionTimerVal: { secs: bigint; warnings: bigint[] } | null = null;
    let teamPreviewTimerVal: { secs: bigint; warnings: bigint[] } | null = null;

    if (timerSettings.preset === "custom") {
      if (timerSettings.battleTimer)
        battleTimerVal = { secs: parseBigIntSafe(timerSettings.battleTimer), warnings: [] };
      if (timerSettings.playerTimer)
        playerTimerVal = { secs: parseBigIntSafe(timerSettings.playerTimer), warnings: [] };
      if (timerSettings.actionTimer)
        actionTimerVal = { secs: parseBigIntSafe(timerSettings.actionTimer), warnings: [] };
      if (timerSettings.teamPreviewTimer)
        teamPreviewTimerVal = {
          secs: parseBigIntSafe(timerSettings.teamPreviewTimer),
          warnings: [],
        };
    } else if (timerSettings.preset !== "none") {
      const preset = TIMER_PRESETS[timerSettings.preset];
      if (preset.battleTimer)
        battleTimerVal = { secs: parseBigIntSafe(preset.battleTimer), warnings: [] };
      if (preset.playerTimer)
        playerTimerVal = { secs: parseBigIntSafe(preset.playerTimer), warnings: [] };
      if (preset.actionTimer)
        actionTimerVal = { secs: parseBigIntSafe(preset.actionTimer), warnings: [] };
      if (preset.teamPreviewTimer)
        teamPreviewTimerVal = { secs: parseBigIntSafe(preset.teamPreviewTimer), warnings: [] };
    }

    const timers = {
      battle: battleTimerVal,
      player: playerTimerVal,
      action: actionTimerVal,
      team_preview: teamPreviewTimerVal,
    };

    // Helper to map FormPlayer to PlayerData bindings format
    const mapFormPlayerToPlayerData = (p: FormPlayer) => {
      const player_type =
        p.playerType === "wild"
          ? {
              type: "wild",
              catchable: p.wildCatchable,
              escapable: p.wildEscapable,
              can_escape: p.wildCanEscape,
              encounter_type: p.wildEncounterType,
            }
          : { type: p.playerType };

      // Map team members if the player is an AI and has a selected team
      let teamMembers: MonData[] = [];
      if (p.controlType === "ai" && p.selectedTeam) {
        teamMembers = teams[p.selectedTeam] || [];
      }

      return {
        id: p.id.trim().toLowerCase(),
        name: p.id.trim(),
        player_type,
        player_options: {
          has_affection: p.hasAffection,
          has_strict_bag: p.hasStrictBag,
          experience: { share: null, custom_modifier: null },
          mons_caught: p.monsCaught,
          cannot_mega_evolve: p.cannotMegaEvolve,
          cannot_z_move: p.cannotZMove,
          cannot_dynamax: p.cannotDynamax,
          cannot_terastallize: p.cannotTerastallize,
        },
        team: { members: teamMembers, bag: { items: {} } },
        dex: { species: [] },
      };
    };

    const side1PlayersData = finalSide1.map(mapFormPlayerToPlayerData);
    const side2PlayersData = finalSide2.map(mapFormPlayerToPlayerData);

    // Dynamic side names formatting: delimiter "/" when multiple players on a side
    const side1Name = side1PlayersData.map((p) => p.name).join(" / ");
    const side2Name = side2PlayersData.map((p) => p.name).join(" / ");

    // Core battle options payload
    const battleOptions = {
      seed: 0n,
      format: {
        battle_type: format,
        rules: rulesArray,
      },
      field: {
        weather: fieldSettings.weather === "None" ? null : fieldSettings.weather,
        terrain: fieldSettings.terrain === "None" ? null : fieldSettings.terrain,
        environment: fieldSettings.environment,
        time: fieldSettings.timeOfDay,
      },
      side_1: {
        name: side1Name,
        players: side1PlayersData,
      },
      side_2: {
        name: side2Name,
        players: side2PlayersData,
      },
    };

    const proposedOptions = {
      battle_options: battleOptions as unknown as CoreBattleOptions,
      service_options: {
        creator: connection.playerId || "",
        timers: timers,
        log_timer_deadlines: true,
      },
      timeout: { secs: timerSettings.proposalTimeout, nanos: 0 },
    };

    dispatch(proposeBattle(proposedOptions))
      .unwrap()
      .then((proposal) => {
        // Reset opponent names on side 2
        setSide2Players([createDefaultPlayer()]);
        dispatch(updateProposal(proposal));
        dispatch(selectBattle({ view: "proposal", battleId: proposal.uuid }));
      })
      .catch((err) => {
        dispatch(setConnectionError("Failed to send proposal: " + (err.message || err), err));
      });
  };

  return (
    <section className="card">
      <div className="card-header">
        <h3>New Battle Proposal</h3>
      </div>
      <form onSubmit={handleSendProposal} className="w-full flex-col gap-m">
        <div className="w-full flex-row flex-wrap gap-m">
          <div className={`form-group ${styles.formatField}`}>
            <label htmlFor="format">Format</label>
            <select
              id="format"
              value={format}
              onChange={(e) => setFormat(e.target.value as BattleType)}
            >
              <option value="Singles">Singles</option>
              <option value="Doubles">Doubles</option>
              <option value="Multi">Multi</option>
              <option value="Triples">Triples</option>
            </select>
          </div>

          {!isAdvancedView && (
            <>
              <div className={`form-group ${styles.opponentControlField}`}>
                <label htmlFor="opponentControlType">Opponent</label>
                <select
                  id="opponentControlType"
                  value={side2Players[0].controlType}
                  onChange={(e) => {
                    const val = e.target.value as "human" | "ai";
                    updatePlayerSlot(2, 0, {
                      controlType: val,
                      id: val === "ai" ? "" : side2Players[0].id,
                      selectedTeam: val === "ai" ? defaultTeam || teamNames[0] || "" : undefined,
                    });
                  }}
                >
                  <option value="human">Player</option>
                  <option value="ai">Random AI</option>
                </select>
              </div>

              {side2Players[0].controlType === "human" ? (
                <div className={`form-group ${styles.opponentField}`}>
                  <label htmlFor="opponentName">Opponent name</label>
                  <input
                    id="opponentName"
                    type="text"
                    value={side2Players[0].id}
                    onChange={(e) => updatePlayerSlot(2, 0, { id: e.target.value })}
                    placeholder="Player name"
                    required
                  />
                </div>
              ) : (
                <div className={`form-group ${styles.opponentField}`}>
                  <label htmlFor="opponentTeam">AI team</label>
                  {teamNames.length > 0 ? (
                    <TeamSelect
                      id="opponentTeam"
                      value={side2Players[0].selectedTeam || ""}
                      onChange={(val) => updatePlayerSlot(2, 0, { selectedTeam: val })}
                      teamNames={teamNames}
                      teams={teams}
                      required
                    />
                  ) : (
                    <span className="text-danger text-sm">No teams. Go to Teams.</span>
                  )}
                </div>
              )}
            </>
          )}
        </div>

        {/* Player slot editors for Side 1 and Side 2 */}
        {isAdvancedView && (
          <div className={styles.sidesContainer}>
            {/* Side 1 */}
            <div className="flex-1 flex-col gap-s">
              <div className="flex-row justify-between align-center">
                <span className={styles.sideHeaderLabel}>Side 1</span>
                {format === "Multi" && side1Players.length < 5 && (
                  <button
                    type="button"
                    className="btn btn-secondary btn-sm"
                    onClick={() => addPlayerSlot(1)}
                  >
                    + Add ally
                  </button>
                )}
              </div>
              <div className="flex-col gap-s">
                {side1Players.map((player, idx) => (
                  <PlayerSlotCard
                    key={`1-${idx}`}
                    side={1}
                    index={idx}
                    player={player}
                    isProposer={idx === 0}
                    format={format}
                    teams={teams}
                    teamNames={teamNames}
                    defaultTeam={defaultTeam}
                    onRemove={() => removePlayerSlot(1, idx)}
                    onChange={(fields) => updatePlayerSlot(1, idx, fields)}
                  />
                ))}
              </div>
            </div>

            {/* Side 2 */}
            <div className="flex-1 flex-col gap-s">
              <div className="flex-row justify-between align-center">
                <span className={styles.sideHeaderLabel}>Side 2</span>
                {format === "Multi" && side2Players.length < 5 && (
                  <button
                    type="button"
                    className="btn btn-secondary btn-sm"
                    onClick={() => addPlayerSlot(2)}
                  >
                    + Add opponent
                  </button>
                )}
              </div>
              <div className="flex-col gap-s">
                {side2Players.map((player, idx) => (
                  <PlayerSlotCard
                    key={`2-${idx}`}
                    side={2}
                    index={idx}
                    player={player}
                    isProposer={false}
                    format={format}
                    teams={teams}
                    teamNames={teamNames}
                    defaultTeam={defaultTeam}
                    onRemove={() => removePlayerSlot(2, idx)}
                    onChange={(fields) => updatePlayerSlot(2, idx, fields)}
                  />
                ))}
              </div>
            </div>
          </div>
        )}

        {/* Advanced options trigger */}
        <div className="flex-row justify-start w-full">
          <button
            type="button"
            className="btn btn-secondary mt-s"
            onClick={() => setShowAdvanced(!showAdvanced)}
          >
            {showAdvanced ? "Hide advanced options" : "Show advanced options"}
          </button>
        </div>

        {/* Advanced Configurations Section */}
        {showAdvanced && (
          <div className="flex-col gap-m w-full border-top pt-m mt-s">
            <AdvancedRulesSection
              customRules={customRules}
              onChange={(fields) => setCustomRules((prev) => ({ ...prev, ...fields }))}
            />

            <FieldSettingsSection
              fieldSettings={fieldSettings}
              onChange={(fields) => setFieldSettings((prev) => ({ ...prev, ...fields }))}
            />

            <TimerSettingsSection
              timerSettings={timerSettings}
              onChange={(fields) => setTimerSettings((prev) => ({ ...prev, ...fields }))}
            />
          </div>
        )}

        <div className="flex-row justify-start w-full mt-s">
          <button type="submit" className="btn btn-primary">
            Propose
          </button>
        </div>
      </form>
    </section>
  );
}
