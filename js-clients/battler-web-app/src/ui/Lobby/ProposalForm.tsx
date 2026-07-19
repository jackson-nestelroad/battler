import React, { useState, useEffect, useMemo } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { proposeBattle } from "../../core/wamp";
import { selectBattle } from "../../store/battlesSlice";
import { updateProposal } from "../../store/proposalsSlice";
import type { CoreBattleOptions, BattleType, FieldEnvironment, TimeOfDay } from "battler-types";

import { setConnectionError } from "../../store/connectionSlice";

import styles from "./ProposalForm.module.scss";

interface FormPlayer {
  id: string;
  controlType: "human" | "ai";
  playerType: "trainer" | "wild" | "protagonist";
  hasAffection: boolean;
  hasStrictBag: boolean;
  cannotMegaEvolve: boolean;
  cannotZMove: boolean;
  cannotDynamax: boolean;
  cannotTerastallize: boolean;
  monsCaught: number;
  // Wild options
  wildCatchable: boolean;
  wildEscapable: boolean;
  wildCanEscape: boolean;
  wildEncounterType: "Normal" | "Fishing";
  selectedTeam?: string;
}

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
    {
      id: connection.playerId || "",
      controlType: "human",
      playerType: "trainer",
      hasAffection: false,
      hasStrictBag: false,
      cannotMegaEvolve: false,
      cannotZMove: false,
      cannotDynamax: false,
      cannotTerastallize: false,
      monsCaught: 0,
      wildCatchable: true,
      wildEscapable: true,
      wildCanEscape: true,
      wildEncounterType: "Normal",
    },
  ]);

  // Side 2 Players (starts with 1 slot)
  const [side2Players, setSide2Players] = useState<FormPlayer[]>([
    {
      id: "",
      controlType: "human",
      playerType: "trainer",
      hasAffection: false,
      hasStrictBag: false,
      cannotMegaEvolve: false,
      cannotZMove: false,
      cannotDynamax: false,
      cannotTerastallize: false,
      monsCaught: 0,
      wildCatchable: true,
      wildEscapable: true,
      wildCanEscape: true,
      wildEncounterType: "Normal",
    },
  ]);

  // Collapsible state for each player settings card
  const [openPlayerSettings, setOpenPlayerSettings] = useState<Record<string, boolean>>({});

  // Form advanced settings states
  const [rulesetPreset, setRulesetPreset] = useState<
    "none" | "standard" | "standarddoubles" | "flatrules" | "custom"
  >("standard");

  // Custom rules toggles
  const [customSpeciesClause, setCustomSpeciesClause] = useState(true);
  const [customSleepClause, setCustomSleepClause] = useState(true);
  const [customItemClause, setCustomItemClause] = useState(true);
  const [customNicknameClause, setCustomNicknameClause] = useState(true);
  const [customOhkoClause, setCustomOhkoClause] = useState(true);
  const [customEvasionClause, setCustomEvasionClause] = useState(true);
  const [customEndlessBattleClause, setCustomEndlessBattleClause] = useState(true);

  // Custom mechanical limits toggles
  const [customMegaEvolution, setCustomMegaEvolution] = useState(false);
  const [customZMoves, setCustomZMoves] = useState(false);
  const [customDynamax, setCustomDynamax] = useState(false);
  const [customTerastallization, setCustomTerastallization] = useState(false);
  const [customBagItems, setCustomBagItems] = useState(false);

  // Custom numeric rules
  const [customPickedTeamSizeAuto, setCustomPickedTeamSizeAuto] = useState<boolean>(true);
  const [customPickedTeamSize, setCustomPickedTeamSize] = useState<number>(3);
  const [customAdjustLevelDownEnabled, setCustomAdjustLevelDownEnabled] = useState<boolean>(false);
  const [customAdjustLevelDown, setCustomAdjustLevelDown] = useState<number>(50);
  const [customDefaultLevel, setCustomDefaultLevel] = useState<number>(50);
  const [customMaxLevel, setCustomMaxLevel] = useState<number>(100);

  // Field settings
  const [weather, setWeather] = useState<string>("None");
  const [terrain, setTerrain] = useState<string>("None");
  const [environment, setEnvironment] = useState<FieldEnvironment>("Normal");
  const [timeOfDay, setTimeOfDay] = useState<TimeOfDay>("Day");

  // Timer settings
  const [timerPreset, setTimerPreset] = useState<"none" | "blitz" | "standard" | "custom">("none");
  const [customBattleTimer, setCustomBattleTimer] = useState<string>("");
  const [customPlayerTimer, setCustomPlayerTimer] = useState<string>("");
  const [customActionTimer, setCustomActionTimer] = useState<string>("");
  const [proposalTimeout, setProposalTimeout] = useState<number>(60);

  // Custom rules list and rules builder form state
  const [customRulesList, setCustomRulesList] = useState<string[]>([]);
  const [ruleAction, setRuleAction] = useState<"clause" | "ban" | "allow" | "repeal">("clause");
  const [ruleCategory, setRuleCategory] = useState<string>("");
  const [ruleValue, setRuleValue] = useState<string>("");

  const handleAddRule = () => {
    const trimmed = ruleValue.trim();
    if (!trimmed) return;
    let formatted = "";
    if (ruleAction === "clause") {
      formatted = trimmed;
    } else if (ruleAction === "repeal") {
      formatted = `! ${trimmed}`;
    } else {
      const prefix = ruleAction === "ban" ? "-" : "+";
      formatted = `${prefix}${ruleCategory ? " " + ruleCategory : ""}: ${trimmed}`;
    }
    if (formatted && !customRulesList.includes(formatted)) {
      setCustomRulesList([...customRulesList, formatted]);
    }
    setRuleValue("");
  };

  const handleRemoveRule = (index: number) => {
    setCustomRulesList(customRulesList.filter((_, idx) => idx !== index));
  };

  const isCustom = rulesetPreset === "custom";

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
          return [
            ...prev,
            {
              id: "",
              controlType: "human",
              playerType: "trainer",
              hasAffection: false,
              hasStrictBag: false,
              cannotMegaEvolve: false,
              cannotZMove: false,
              cannotDynamax: false,
              cannotTerastallize: false,
              monsCaught: 0,
              wildCatchable: true,
              wildEscapable: true,
              wildCanEscape: true,
              wildEncounterType: "Normal",
            },
          ];
        }
        return prev;
      });
      setSide2Players((prev) => {
        if (prev.length < 2) {
          return [
            ...prev,
            {
              id: "",
              controlType: "human",
              playerType: "trainer",
              hasAffection: false,
              hasStrictBag: false,
              cannotMegaEvolve: false,
              cannotZMove: false,
              cannotDynamax: false,
              cannotTerastallize: false,
              monsCaught: 0,
              wildCatchable: true,
              wildEscapable: true,
              wildCanEscape: true,
              wildEncounterType: "Normal",
            },
          ];
        }
        return prev;
      });
    }
  }, [format]);

  const addPlayerSlot = (side: 1 | 2) => {
    const newPlayer: FormPlayer = {
      id: "",
      controlType: "human",
      playerType: "trainer",
      hasAffection: false,
      hasStrictBag: false,
      cannotMegaEvolve: false,
      cannotZMove: false,
      cannotDynamax: false,
      cannotTerastallize: false,
      monsCaught: 0,
      wildCatchable: true,
      wildEscapable: true,
      wildCanEscape: true,
      wildEncounterType: "Normal",
    };
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

  const togglePlayerSettings = (key: string) => {
    setOpenPlayerSettings((prev) => ({
      ...prev,
      [key]: !prev[key],
    }));
  };

  const getActionTimerValue = () => {
    if (timerPreset === "custom") return customActionTimer;
    if (timerPreset === "blitz") return "15";
    if (timerPreset === "standard") return "45";
    return "";
  };

  const getPlayerTimerValue = () => {
    if (timerPreset === "custom") return customPlayerTimer;
    if (timerPreset === "standard") return "420";
    return "";
  };

  const getBattleTimerValue = () => {
    if (timerPreset === "custom") return customBattleTimer;
    if (timerPreset === "standard") return "1200";
    return "";
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

    // Check if any AI player does not have a selected team
    const hasEmptyAiTeam =
      finalSide1.some((p) => p.controlType === "ai" && !p.selectedTeam) ||
      finalSide2.some((p) => p.controlType === "ai" && !p.selectedTeam);

    if (hasEmptyAiTeam) {
      dispatch(setConnectionError("Please select a team for all AI players.", null));
      return;
    }

    // Build format rules array
    let rulesArray: string[] = [];
    if (rulesetPreset === "standard") {
      rulesArray = ["Standard"];
    } else if (rulesetPreset === "standarddoubles") {
      rulesArray = ["Standard Doubles"];
    } else if (rulesetPreset === "flatrules") {
      rulesArray = ["Flat Rules"];
    } else if (rulesetPreset === "custom") {
      if (customSleepClause) rulesArray.push("Sleep Clause");
      if (customSpeciesClause) rulesArray.push("Species Clause");
      if (customItemClause) rulesArray.push("Item Clause");
      if (customNicknameClause) rulesArray.push("Nickname Clause");
      if (customOhkoClause) rulesArray.push("OHKO Clause");
      if (customEvasionClause) {
        rulesArray.push("Evasion Items Clause");
        rulesArray.push("Evasion Moves Clause");
      }
      if (customEndlessBattleClause) rulesArray.push("Endless Battle Clause");

      // Custom rule flags (Dynamax, Z-Moves, Mega Evolution, Terastallization)
      if (customMegaEvolution) rulesArray.push("Mega Evolution");
      if (customZMoves) rulesArray.push("Z-Moves");
      if (customDynamax) rulesArray.push("Dynamax");
      if (customTerastallization) rulesArray.push("Terastallization");
      if (customBagItems) rulesArray.push("Bag Items");

      if (!customPickedTeamSizeAuto) {
        rulesArray.push(`Picked Team Size = ${customPickedTeamSize}`);
      }
      if (customAdjustLevelDownEnabled) {
        rulesArray.push(`Adjust Level Down = ${customAdjustLevelDown}`);
      }
      if (customDefaultLevel) rulesArray.push(`Default Level = ${customDefaultLevel}`);
      if (customMaxLevel) rulesArray.push(`Max Level = ${customMaxLevel}`);
      for (const rule of customRulesList) {
        rulesArray.push(rule);
      }
    }

    // TODO: Re-enable when we support Team Preview.
    rulesArray.push("! Team Preview");

    // Build timers
    let battleTimerVal: { secs: bigint; warnings: bigint[] } | null = null;
    let playerTimerVal: { secs: bigint; warnings: bigint[] } | null = null;
    let actionTimerVal: { secs: bigint; warnings: bigint[] } | null = null;

    if (timerPreset === "blitz") {
      actionTimerVal = { secs: 15n, warnings: [] };
    } else if (timerPreset === "standard") {
      actionTimerVal = { secs: 45n, warnings: [] };
      playerTimerVal = { secs: 420n, warnings: [] };
      battleTimerVal = { secs: 1200n, warnings: [] };
    } else if (timerPreset === "custom") {
      if (customBattleTimer) battleTimerVal = { secs: BigInt(customBattleTimer), warnings: [] };
      if (customPlayerTimer) playerTimerVal = { secs: BigInt(customPlayerTimer), warnings: [] };
      if (customActionTimer) actionTimerVal = { secs: BigInt(customActionTimer), warnings: [] };
    }

    const timers = {
      battle: battleTimerVal,
      player: playerTimerVal,
      action: actionTimerVal,
    };

    // Helper to map FormPlayer to PlayerData bindings format
    const mapFormPlayerToPlayerData = (p: FormPlayer) => {
      let player_type: any = { type: p.playerType };
      if (p.playerType === "wild") {
        player_type = {
          type: "wild",
          catchable: p.wildCatchable,
          escapable: p.wildEscapable,
          can_escape: p.wildCanEscape,
          encounter_type: p.wildEncounterType,
        };
      }

      // Map team members if the player is an AI and has a selected team
      let teamMembers: any[] = [];
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
        weather: weather === "None" ? null : weather,
        terrain: terrain === "None" ? null : terrain,
        environment: environment,
        time: timeOfDay,
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
      },
      timeout: { secs: proposalTimeout, nanos: 0 },
    };

    dispatch(proposeBattle(proposedOptions))
      .unwrap()
      .then((proposal) => {
        // Reset opponent names on side 2
        setSide2Players([
          {
            id: "",
            controlType: "human",
            playerType: "trainer",
            hasAffection: false,
            hasStrictBag: false,
            cannotMegaEvolve: false,
            cannotZMove: false,
            cannotDynamax: false,
            cannotTerastallize: false,
            monsCaught: 0,
            wildCatchable: true,
            wildEscapable: true,
            wildCanEscape: true,
            wildEncounterType: "Normal",
          },
        ]);
        dispatch(updateProposal(proposal));
        dispatch(selectBattle({ view: "proposal", battleId: proposal.uuid }));
      })
      .catch((err) => {
        dispatch(setConnectionError("Failed to send proposal: " + (err.message || err), err));
      });
  };

  const renderPlayerSlot = (side: 1 | 2, index: number, player: FormPlayer) => {
    const isProposer = side === 1 && index === 0;
    const slotKey = `${side}-${index}`;
    const isOpen = !!openPlayerSettings[slotKey];

    return (
      <div key={slotKey} className={styles.playerSlotCard}>
        <div className="flex-row justify-between align-center mb-xs">
          <span className={styles.playerSlotLabel}>
            {isProposer ? "You (Proposer)" : `Player #${index + 1}`}
          </span>
          {!isProposer && format === "Multi" && (
            <button
              type="button"
              className="btn btn-danger btn-sm"
              onClick={() => removePlayerSlot(side, index)}
            >
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
                  updatePlayerSlot(side, index, {
                    controlType: val,
                    id: val === "ai" ? "" : player.id,
                    selectedTeam: val === "ai" ? (defaultTeam || teamNames[0] || "") : undefined,
                  });
                }}
                disabled={isProposer}
              >
                <option value="human">Human</option>
                <option value="ai">AI</option>
              </select>
            </div>

            {player.controlType === "human" ? (
              <div className="form-group flex-1">
                <label>Player name</label>
                <input
                  type="text"
                  value={player.id}
                  onChange={(e) => updatePlayerSlot(side, index, { id: e.target.value })}
                  placeholder="Username"
                  disabled={isProposer}
                  required
                />
              </div>
            ) : (
              <div className="form-group flex-1">
                <label>AI Type</label>
                <select value="random" disabled>
                  <option value="random">Random AI</option>
                </select>
              </div>
            )}

            <div className="form-group flex-1">
              <label>Player Type</label>
              <select
                value={player.playerType}
                onChange={(e) =>
                  updatePlayerSlot(side, index, {
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
                <label>AI Team</label>
                {teamNames.length > 0 ? (
                  <select
                    value={player.selectedTeam || ""}
                    onChange={(e) =>
                      updatePlayerSlot(side, index, { selectedTeam: e.target.value })
                    }
                    required
                  >
                    {teamNames.map((name) => (
                      <option key={name} value={name}>
                        {name} ({teams[name].length})
                      </option>
                    ))}
                  </select>
                ) : (
                  <span className="text-danger text-sm">
                    No teams. Go to Teams.
                  </span>
                )}
              </div>
            )}
          </div>

          <div>
            <button
              type="button"
              className={`btn btn-secondary btn-sm ${styles.advancedSettingsToggle}`}
              onClick={() => togglePlayerSettings(slotKey)}
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
                    onChange={(e) =>
                      updatePlayerSlot(side, index, { hasAffection: e.target.checked })
                    }
                  />
                  <span>Affection</span>
                </label>

                <label className={styles.checkboxLabel}>
                  <input
                    type="checkbox"
                    checked={player.hasStrictBag}
                    onChange={(e) =>
                      updatePlayerSlot(side, index, { hasStrictBag: e.target.checked })
                    }
                  />
                  <span>Strict bag</span>
                </label>

                <label className={styles.checkboxLabel}>
                  <input
                    type="checkbox"
                    checked={!player.cannotMegaEvolve}
                    onChange={(e) =>
                      updatePlayerSlot(side, index, { cannotMegaEvolve: !e.target.checked })
                    }
                  />
                  <span>Mega Evolution</span>
                </label>

                <label className={styles.checkboxLabel}>
                  <input
                    type="checkbox"
                    checked={!player.cannotZMove}
                    onChange={(e) =>
                      updatePlayerSlot(side, index, { cannotZMove: !e.target.checked })
                    }
                  />
                  <span>Z-Moves</span>
                </label>

                <label className={styles.checkboxLabel}>
                  <input
                    type="checkbox"
                    checked={!player.cannotDynamax}
                    onChange={(e) =>
                      updatePlayerSlot(side, index, { cannotDynamax: !e.target.checked })
                    }
                  />
                  <span>Dynamax</span>
                </label>

                <label className={styles.checkboxLabel}>
                  <input
                    type="checkbox"
                    checked={!player.cannotTerastallize}
                    onChange={(e) =>
                      updatePlayerSlot(side, index, { cannotTerastallize: !e.target.checked })
                    }
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
                    onChange={(e) =>
                      updatePlayerSlot(side, index, { monsCaught: Number(e.target.value) })
                    }
                  />
                </div>
                {player.playerType === "wild" && (
                  <div className="form-group flex-1">
                    <label>Encounter type</label>
                    <select
                      value={player.wildEncounterType}
                      onChange={(e) =>
                        updatePlayerSlot(side, index, {
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
                      onChange={(e) =>
                        updatePlayerSlot(side, index, { wildCatchable: e.target.checked })
                      }
                    />
                    <span>Catchable</span>
                  </label>

                  <label className={styles.checkboxLabel}>
                    <input
                      type="checkbox"
                      checked={player.wildEscapable}
                      onChange={(e) =>
                        updatePlayerSlot(side, index, { wildEscapable: e.target.checked })
                      }
                    />
                    <span>Escapable</span>
                  </label>

                  <label className={styles.checkboxLabel}>
                    <input
                      type="checkbox"
                      checked={player.wildCanEscape}
                      onChange={(e) =>
                        updatePlayerSlot(side, index, { wildCanEscape: e.target.checked })
                      }
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
  };

  return (
    <section className="card">
      <div className="card-header">
        <h3>New Battle Proposal</h3>
      </div>
      <form onSubmit={handleSendProposal} className={`${styles.proposalForm} flex-col gap-m`}>
        <div className={`${styles.formFields} flex-row gap-m`}>
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
        </div>

        {/* Player slot editors for Side 1 and Side 2 */}
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
              {side1Players.map((p, idx) => renderPlayerSlot(1, idx, p))}
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
              {side2Players.map((p, idx) => renderPlayerSlot(2, idx, p))}
            </div>
          </div>
        </div>

        {/* Advanced options trigger */}
        <div className="flex-row justify-start w-full">
          <button
            type="button"
            className={`btn btn-secondary ${styles.advancedToggle}`}
            onClick={() => setShowAdvanced(!showAdvanced)}
          >
            {showAdvanced ? "Hide advanced options" : "Show advanced options"}
          </button>
        </div>

        {/* Advanced Configurations Section */}
        {showAdvanced && (
          <div className="flex-col gap-m w-full border-top pt-m mt-s">
            {/* Rules preset and custom clauses */}
            <div className={styles.advancedSection}>
              <h4 className="mb-s">Format Rules</h4>
              <div className="flex-row flex-mobile-col gap-m align-end">
                <div className="form-group flex-1">
                  <label htmlFor="rulesetPreset">Ruleset preset</label>
                  <select
                    id="rulesetPreset"
                    value={rulesetPreset}
                    onChange={(e) => setRulesetPreset(e.target.value as any)}
                  >
                    <option value="none">None</option>
                    <option value="standard">Standard</option>
                    <option value="standarddoubles">Standard Doubles</option>
                    <option value="flatrules">Flat Rules</option>
                    <option value="custom">Custom...</option>
                  </select>
                </div>
              </div>

              {isCustom && (
                <div className="flex-col gap-m mt-m">
                  <div className={styles.checkboxGrid}>
                    <label className={styles.checkboxLabel}>
                      <input
                        type="checkbox"
                        checked={customSpeciesClause}
                        onChange={(e) => setCustomSpeciesClause(e.target.checked)}
                      />
                      <span>Species Clause</span>
                    </label>

                    <label className={styles.checkboxLabel}>
                      <input
                        type="checkbox"
                        checked={customSleepClause}
                        onChange={(e) => setCustomSleepClause(e.target.checked)}
                      />
                      <span>Sleep Clause</span>
                    </label>

                    <label className={styles.checkboxLabel}>
                      <input
                        type="checkbox"
                        checked={customItemClause}
                        onChange={(e) => setCustomItemClause(e.target.checked)}
                      />
                      <span>Item Clause</span>
                    </label>

                    <label className={styles.checkboxLabel}>
                      <input
                        type="checkbox"
                        checked={customNicknameClause}
                        onChange={(e) => setCustomNicknameClause(e.target.checked)}
                      />
                      <span>Nickname Clause</span>
                    </label>

                    <label className={styles.checkboxLabel}>
                      <input
                        type="checkbox"
                        checked={customOhkoClause}
                        onChange={(e) => setCustomOhkoClause(e.target.checked)}
                      />
                      <span>OHKO Clause</span>
                    </label>

                    <label className={styles.checkboxLabel}>
                      <input
                        type="checkbox"
                        checked={customEvasionClause}
                        onChange={(e) => setCustomEvasionClause(e.target.checked)}
                      />
                      <span>Evasion Clause</span>
                    </label>

                    <label className={styles.checkboxLabel}>
                      <input
                        type="checkbox"
                        checked={customEndlessBattleClause}
                        onChange={(e) => setCustomEndlessBattleClause(e.target.checked)}
                      />
                      <span>Endless Battle Clause</span>
                    </label>

                    <label className={styles.checkboxLabel}>
                      <input
                        type="checkbox"
                        checked={customMegaEvolution}
                        onChange={(e) => setCustomMegaEvolution(e.target.checked)}
                      />
                      <span>Mega Evolution</span>
                    </label>

                    <label className={styles.checkboxLabel}>
                      <input
                        type="checkbox"
                        checked={customZMoves}
                        onChange={(e) => setCustomZMoves(e.target.checked)}
                      />
                      <span>Z-Moves</span>
                    </label>

                    <label className={styles.checkboxLabel}>
                      <input
                        type="checkbox"
                        checked={customDynamax}
                        onChange={(e) => setCustomDynamax(e.target.checked)}
                      />
                      <span>Dynamax</span>
                    </label>

                    <label className={styles.checkboxLabel}>
                      <input
                        type="checkbox"
                        checked={customTerastallization}
                        onChange={(e) => setCustomTerastallization(e.target.checked)}
                      />
                      <span>Terastallization</span>
                    </label>

                    <label className={styles.checkboxLabel}>
                      <input
                        type="checkbox"
                        checked={customBagItems}
                        onChange={(e) => setCustomBagItems(e.target.checked)}
                      />
                      <span>Bag items</span>
                    </label>
                  </div>

                  <div className="flex-row flex-mobile-col gap-m mt-s">
                    <div className="form-group flex-1">
                      <label htmlFor="customPickedTeamSize">Picked team size</label>
                      <input
                        id="customPickedTeamSize"
                        type="number"
                        min="1"
                        max="6"
                        value={customPickedTeamSizeAuto ? "" : customPickedTeamSize}
                        onChange={(e) => setCustomPickedTeamSize(Number(e.target.value))}
                        placeholder={customPickedTeamSizeAuto ? "Auto" : undefined}
                        disabled={customPickedTeamSizeAuto}
                      />
                      <label
                        className={styles.checkboxLabel}
                        style={{ marginTop: "var(--spacing-xs)" }}
                      >
                        <input
                          type="checkbox"
                          checked={customPickedTeamSizeAuto}
                          onChange={(e) => setCustomPickedTeamSizeAuto(e.target.checked)}
                        />
                        <span>Auto</span>
                      </label>
                    </div>

                    <div className="form-group flex-1">
                      <label htmlFor="customAdjustLevelDown">Adjust level down</label>
                      <input
                        id="customAdjustLevelDown"
                        type="number"
                        min="1"
                        max="100"
                        value={customAdjustLevelDownEnabled ? customAdjustLevelDown : ""}
                        onChange={(e) => setCustomAdjustLevelDown(Number(e.target.value))}
                        placeholder={customAdjustLevelDownEnabled ? undefined : "None"}
                        disabled={!customAdjustLevelDownEnabled}
                      />
                      <label
                        className={styles.checkboxLabel}
                        style={{ marginTop: "var(--spacing-xs)" }}
                      >
                        <input
                          type="checkbox"
                          checked={customAdjustLevelDownEnabled}
                          onChange={(e) => setCustomAdjustLevelDownEnabled(e.target.checked)}
                        />
                        <span>Enable custom limit</span>
                      </label>
                    </div>

                    <div className="form-group flex-1">
                      <label htmlFor="customDefaultLevel">Default level</label>
                      <input
                        id="customDefaultLevel"
                        type="number"
                        min="1"
                        max="100"
                        value={customDefaultLevel}
                        onChange={(e) => setCustomDefaultLevel(Number(e.target.value))}
                      />
                    </div>

                    <div className="form-group flex-1">
                      <label htmlFor="customMaxLevel">Max level</label>
                      <input
                        id="customMaxLevel"
                        type="number"
                        min="1"
                        max="100"
                        value={customMaxLevel}
                        onChange={(e) => setCustomMaxLevel(Number(e.target.value))}
                      />
                    </div>
                  </div>

                  {/* Active Rules List */}
                  <div className="flex-col gap-s w-full border-top pt-m mt-m">
                    <span
                      className={styles.sideHeaderLabel}
                      style={{ fontSize: "var(--font-size-s)" }}
                    >
                      Other Rules
                    </span>
                    <div className="flex-row flex-wrap gap-xs mt-xs">
                      {customRulesList.map((rule, idx) => (
                        <div key={idx} className={styles.ruleBadge}>
                          <span>{rule}</span>
                          <button
                            type="button"
                            className={styles.removeRuleBtn}
                            onClick={() => handleRemoveRule(idx)}
                          >
                            &times;
                          </button>
                        </div>
                      ))}
                      {customRulesList.length === 0 && (
                        <span className="text-secondary italic">None</span>
                      )}
                    </div>
                  </div>

                  {/* Rules Builder Form */}
                  <div className={styles.rulesBuilderRow}>
                    <div className={`form-group ${styles.actionField}`}>
                      <label htmlFor="ruleAction">Action</label>
                      <select
                        id="ruleAction"
                        value={ruleAction}
                        onChange={(e) => setRuleAction(e.target.value as any)}
                      >
                        <option value="clause">Clause</option>
                        <option value="ban">Ban (-)</option>
                        <option value="allow">Allow (+)</option>
                        <option value="repeal">Repeal (!)</option>
                      </select>
                    </div>

                    {(ruleAction === "ban" || ruleAction === "allow") && (
                      <div className={`form-group ${styles.categoryField}`}>
                        <label htmlFor="ruleCategory">Category</label>
                        <select
                          id="ruleCategory"
                          value={ruleCategory}
                          onChange={(e) => setRuleCategory(e.target.value)}
                        >
                          <option value="">None</option>
                          <option value="Move Tag">Move Tag</option>
                          <option value="Item Tag">Item Tag</option>
                          <option value="Ability Tag">Ability Tag</option>
                        </select>
                      </div>
                    )}

                    <div className={`form-group ${styles.valueField}`}>
                      <label htmlFor="ruleValue">Value</label>
                      <input
                        id="ruleValue"
                        type="text"
                        placeholder={
                          ruleAction === "clause"
                            ? "e.g., Same Type Clause"
                            : ruleAction === "repeal"
                              ? "e.g., Sleep Clause"
                              : "e.g., Thunderbolt or Pikachu"
                        }
                        value={ruleValue}
                        onChange={(e) => setRuleValue(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") {
                            e.preventDefault();
                            handleAddRule();
                          }
                        }}
                      />
                    </div>

                    <button
                      type="button"
                      className={`btn btn-secondary ${styles.addBtnField}`}
                      onClick={handleAddRule}
                      disabled={!ruleValue.trim()}
                    >
                      + Add Rule
                    </button>
                  </div>
                </div>
              )}
            </div>

            {/* Weather and terrain starting properties */}
            <div className={styles.advancedSection}>
              <h4 className="mb-s">Field</h4>
              <div className="flex-row flex-mobile-col gap-m">
                <div className="form-group flex-1">
                  <label htmlFor="weather">Default weather</label>
                  <select id="weather" value={weather} onChange={(e) => setWeather(e.target.value)}>
                    <option value="None">None</option>
                    <option value="rainweather">Rain</option>
                    <option value="harshsunlight">Harsh Sunlight</option>
                    <option value="sandstormweather">Sandstorm</option>
                    <option value="hailweather">Hail</option>
                    <option value="snowweather">Snow</option>
                    <option value="heavyrainweather">Heavy Rain</option>
                    <option value="extremelyharshsunlight">Extremely Harsh Sunlight</option>
                    <option value="strongwinds">Strong Winds</option>
                  </select>
                </div>

                <div className="form-group flex-1">
                  <label htmlFor="terrain">Default terrain</label>
                  <select id="terrain" value={terrain} onChange={(e) => setTerrain(e.target.value)}>
                    <option value="None">None</option>
                    <option value="electricterrain">Electric Terrain</option>
                    <option value="grassyterrain">Grassy Terrain</option>
                    <option value="mistyterrain">Misty Terrain</option>
                    <option value="psychicterrain">Psychic Terrain</option>
                  </select>
                </div>

                <div className="form-group flex-1">
                  <label htmlFor="environment">Environment</label>
                  <select
                    id="environment"
                    value={environment}
                    onChange={(e) => setEnvironment(e.target.value as FieldEnvironment)}
                  >
                    <option value="Normal">Normal</option>
                    <option value="Cave">Cave</option>
                    <option value="Sand">Sand</option>
                    <option value="Water">Water</option>
                    <option value="Ice">Ice</option>
                    <option value="Sky">Sky</option>
                    <option value="Grass">Grass</option>
                    <option value="Volcano">Volcano</option>
                  </select>
                </div>

                <div className="form-group flex-1">
                  <label htmlFor="timeOfDay">Time of day</label>
                  <select
                    id="timeOfDay"
                    value={timeOfDay}
                    onChange={(e) => setTimeOfDay(e.target.value as TimeOfDay)}
                  >
                    <option value="Day">Day</option>
                    <option value="Morning">Morning</option>
                    <option value="Evening">Evening</option>
                    <option value="Night">Night</option>
                  </select>
                </div>
              </div>
            </div>

            {/* Match turn action and bank timers */}
            <div className={styles.advancedSection}>
              <h4 className="mb-s">Match Timers</h4>
              <div className="flex-row flex-mobile-col gap-m align-end">
                <div className="form-group flex-1">
                  <label htmlFor="timerPreset">Timer preset</label>
                  <select
                    id="timerPreset"
                    value={timerPreset}
                    onChange={(e) => setTimerPreset(e.target.value as any)}
                  >
                    <option value="none">None</option>
                    <option value="blitz">Blitz</option>
                    <option value="standard">Standard</option>
                    <option value="custom">Custom...</option>
                  </select>
                </div>

                <div className="form-group flex-1">
                  <label htmlFor="proposalTimeout">Proposal timeout (Seconds)</label>
                  <input
                    id="proposalTimeout"
                    type="number"
                    min="10"
                    value={proposalTimeout}
                    onChange={(e) => setProposalTimeout(Number(e.target.value))}
                  />
                </div>
              </div>

              <div className="flex-row flex-mobile-col gap-m mt-m">
                <div className="form-group flex-1">
                  <label htmlFor="customActionTimer">Action timer (Seconds)</label>
                  <input
                    id="customActionTimer"
                    type="number"
                    min="5"
                    placeholder={timerPreset === "custom" ? "e.g., 45" : "None"}
                    value={getActionTimerValue()}
                    onChange={
                      timerPreset === "custom"
                        ? (e) => setCustomActionTimer(e.target.value)
                        : undefined
                    }
                    disabled={timerPreset !== "custom"}
                  />
                </div>

                <div className="form-group flex-1">
                  <label htmlFor="customPlayerTimer">Player timer (Seconds)</label>
                  <input
                    id="customPlayerTimer"
                    type="number"
                    min="10"
                    placeholder={timerPreset === "custom" ? "e.g., 300" : "None"}
                    value={getPlayerTimerValue()}
                    onChange={
                      timerPreset === "custom"
                        ? (e) => setCustomPlayerTimer(e.target.value)
                        : undefined
                    }
                    disabled={timerPreset !== "custom"}
                  />
                </div>

                <div className="form-group flex-1">
                  <label htmlFor="customBattleTimer">Overall match timer (Seconds)</label>
                  <input
                    id="customBattleTimer"
                    type="number"
                    min="30"
                    placeholder={timerPreset === "custom" ? "e.g., 1200" : "None"}
                    value={getBattleTimerValue()}
                    onChange={
                      timerPreset === "custom"
                        ? (e) => setCustomBattleTimer(e.target.value)
                        : undefined
                    }
                    disabled={timerPreset !== "custom"}
                  />
                </div>
              </div>
            </div>
          </div>
        )}

        <div className={styles.formActions}>
          <button type="submit" className="btn btn-primary">
            Propose
          </button>
        </div>
      </form>
    </section>
  );
}
