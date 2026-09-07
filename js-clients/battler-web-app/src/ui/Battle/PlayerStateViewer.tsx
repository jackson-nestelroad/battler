import { Fragment } from "react";
import type { BattleState, Player } from "battler-state";
import { stateSelectors } from "battler-state";
import type { PlayerBattleData } from "battler-types";
import { computeHpPercentage, getMonDisplayName, normalizeStatusCode } from "../../utils/monHelpers";
import { isMonActiveOnField } from "../../utils/battleState";
import { resolveAppearanceRef } from "../../utils/monTooltipModel";
import MonCard from "../Common/MonCard";
import styles from "./PlayerStateViewer.module.scss";

export interface PlayerStateViewerProps {
  battleState?: BattleState | null;
  playerData?: PlayerBattleData | null;
  allyPlayerData?: Record<string, PlayerBattleData> | null;
  localPlayerId?: string | null;
  rules?: string[] | null;
}

function getPlayerCounts(
  player: Player,
  sideIdx: number,
  battleState: BattleState,
  targetPlayerData: PlayerBattleData | null | undefined,
) {
  const hasPrivateTeam = Boolean(targetPlayerData?.mons?.length);

  if (hasPrivateTeam && targetPlayerData) {
    const aliveCount = targetPlayerData.mons.filter(
      (m) => (m.hp ?? 0) > 0 && normalizeStatusCode(m.status) !== "fnt",
    ).length;
    return {
      hasPrivateTeam,
      aliveCount,
      totalCount: targetPlayerData.mons.length,
    };
  }

  const teamSize = player.team_size || player.mons.length;

  const faintedCount = player.mons.filter((m, monIdx) => {
    if (!m.brought) return false;
    if (m.fainted) return true;
    try {
      const ref = resolveAppearanceRef(battleState, player.id, monIdx, m, sideIdx);
      return stateSelectors.monIsFainted(battleState, ref);
    } catch {
      return false;
    }
  }).length;

  return {
    hasPrivateTeam: false,
    aliveCount: Math.max(0, teamSize - faintedCount),
    totalCount: teamSize,
  };
}

function findMatchingPlayerMonIndex(
  mon: NonNullable<PlayerBattleData["mons"]>[number],
  playerMons: Player["mons"],
  matchedIndices: Set<number>,
): number {
  const bSpecies = (mon.species || "").toLowerCase();
  const bName = (mon.summary?.name || mon.species || "").toLowerCase();
  const bGender = mon.summary?.gender;
  const bShiny = mon.summary?.shiny;

  // 1. Exact match on physical appearance
  let matchIdx = playerMons.findIndex((m, mIdx) => {
    if (matchedIndices.has(mIdx)) return false;
    const phys = m.physical_appearance;
    if (!phys) return false;
    const mSpecies = (phys.species || "").toLowerCase();
    const mName = (phys.name || phys.species || "").toLowerCase();
    const speciesMatch = mSpecies === bSpecies;
    const nameMatch = mName === bName;
    const genderMatch =
      bGender == null || phys.gender == null || phys.gender === bGender;
    const shinyMatch =
      bShiny == null || phys.shiny == null || phys.shiny === bShiny;
    return speciesMatch && nameMatch && genderMatch && shinyMatch;
  });

  // 2. Fallback: match on species and name
  if (matchIdx === -1) {
    matchIdx = playerMons.findIndex((m, mIdx) => {
      if (matchedIndices.has(mIdx)) return false;
      const phys = m.physical_appearance;
      if (!phys) return false;
      return (
        (phys.species || "").toLowerCase() === bSpecies &&
        (phys.name || phys.species || "").toLowerCase() === bName
      );
    });
  }

  // 3. Fallback: match on species only
  if (matchIdx === -1) {
    matchIdx = playerMons.findIndex((m, mIdx) => {
      if (matchedIndices.has(mIdx)) return false;
      const phys = m.physical_appearance;
      if (!phys) return false;
      return (phys.species || "").toLowerCase() === bSpecies;
    });
  }

  return matchIdx;
}

function renderPlayerLabel(player: Player, isLocalPlayer: boolean) {
  return (
    <div className="flex-row align-center gap-xs">
      <span className={styles.playerName}>{player.name}</span>
      {isLocalPlayer && <span className="badge badge-primary">You</span>}
      {player.left_battle && (
        <span className={styles.withdrewText}>(withdrew)</span>
      )}
    </div>
  );
}

function renderUnbroughtMonRow(
  m: Player["mons"][number],
  monIdx: number,
  playerId: string,
  sideIdx: number,
  battleState: BattleState,
  rules?: string[] | null,
  keyPrefix = "unbrought",
) {
  const monRef = resolveAppearanceRef(battleState, playerId, monIdx, m, sideIdx);
  const hasBattleAppearance = (m.battle_appearances?.length ?? 0) > 0;
  const phys = m.physical_appearance;
  const name = phys?.name || phys?.species || "Mon";
  let level = 50;
  if (hasBattleAppearance) {
    try {
      level = stateSelectors.monLevel(battleState, monRef) ?? 50;
    } catch {
      level = 50;
    }
  }

  return (
    <MonCard
      key={`${keyPrefix}-${monIdx}`}
      name={name}
      level={level}
      hp={100}
      maxHp={100}
      hpText="100%"
      status={null}
      active={false}
      isUnbrought={true}
      appearanceRef={monRef}
      battleState={battleState}
      rules={rules}
      variant="row"
    />
  );
}

function renderSideConditions(conditions: string[]) {
  if (conditions.length === 0) return null;
  return conditions.map((cond) => (
    <span key={cond} className="badge badge-info">
      {cond}
    </span>
  ));
}

function renderPrivatePlayerTeam(
  targetPlayerData: PlayerBattleData,
  player: Player,
  sideIdx: number,
  battleState: BattleState,
  rules?: string[] | null,
) {
  const matchedStateIndices = new Set<number>();
  const broughtCards = targetPlayerData.mons.map((mon, idx) => {
    const matchIdx = findMatchingPlayerMonIndex(
      mon,
      player.mons,
      matchedStateIndices,
    );

    if (matchIdx !== -1) {
      matchedStateIndices.add(matchIdx);
    }

    const isMonActive =
      matchIdx !== -1
        ? isMonActiveOnField(
            battleState,
            sideIdx,
            player.id,
            matchIdx,
            mon.active,
          )
        : Boolean(mon.active);

    return (
      <MonCard
        key={`brought-${idx}`}
        name={getMonDisplayName(mon) || "Mon"}
        level={mon.summary?.level ?? 50}
        hp={mon.hp}
        maxHp={mon.max_hp}
        status={mon.status}
        active={isMonActive}
        monBattleData={mon}
        battleState={battleState}
        rules={rules}
        variant="row"
      />
    );
  });

  const unbroughtMons = player.mons
    .map((m, monIdx) => ({ m, monIdx }))
    .filter(({ monIdx }) => !matchedStateIndices.has(monIdx));

  return (
    <div className="flex-col gap-xs">
      {broughtCards}
      {unbroughtMons.map(({ m, monIdx }) =>
        renderUnbroughtMonRow(
          m,
          monIdx,
          player.id,
          sideIdx,
          battleState,
          rules,
          "unbrought",
        ),
      )}
    </div>
  );
}

function renderPublicPlayerTeam(
  player: Player,
  sideIdx: number,
  battleState: BattleState,
  rules?: string[] | null,
) {
  const teamSize = player.team_size || player.mons.length;
  const unrevealedCount = Math.max(0, teamSize - player.mons.length);

  return (
    <div className="flex-col gap-xs">
      {player.mons.map((m, monIdx) => {
        const isBrought = Boolean(m.brought);
        if (!isBrought) {
          return renderUnbroughtMonRow(
            m,
            monIdx,
            player.id,
            sideIdx,
            battleState,
            rules,
            "mon",
          );
        }

        const monRef = resolveAppearanceRef(
          battleState,
          player.id,
          monIdx,
          m,
          sideIdx,
        );
        const hasBattleAppearance = (m.battle_appearances?.length ?? 0) > 0;
        let phys = m.physical_appearance;
        let name = phys?.name || phys?.species || "Mon";
        let level = 50;
        let status: string | null = null;
        let isFainted = m.fainted;
        let isActive = false;
        let health: [number, number] | null = null;

        try {
          phys =
            stateSelectors.monPhysicalAppearance(battleState, monRef) || phys;
          name =
            phys?.name ||
            phys?.species ||
            stateSelectors.monSpecies(battleState, monRef) ||
            name;
          level = hasBattleAppearance
            ? (stateSelectors.monLevel(battleState, monRef) ?? 50)
            : 50;
          status = hasBattleAppearance
            ? stateSelectors.monStatus(battleState, monRef)
            : null;
          isFainted =
            m.fainted || stateSelectors.monIsFainted(battleState, monRef);
          isActive = isMonActiveOnField(
            battleState,
            sideIdx,
            player.id,
            monIdx,
          );
          health = hasBattleAppearance
            ? stateSelectors.monHealth(battleState, monRef)
            : null;
        } catch {
          // Fallback to basic appearance properties
        }

        let hp = 100;
        let maxHp = 100;
        let hpText = "100%";

        if (health) {
          const pct = computeHpPercentage(health[0], health[1]);
          hp = pct;
          maxHp = 100;
          hpText = `${pct}%`;
        } else if (isFainted) {
          hp = 0;
          maxHp = 100;
          hpText = "0%";
        }

        return (
          <MonCard
            key={monIdx}
            name={name}
            level={level}
            hp={hp}
            maxHp={maxHp}
            hpText={hpText}
            status={status}
            active={isActive}
            isUnbrought={false}
            appearanceRef={monRef}
            battleState={battleState}
            rules={rules}
            variant="row"
          />
        );
      })}

      {Array.from({ length: unrevealedCount }).map((_, uIdx) => (
        <MonCard
          key={`unrevealed-${uIdx}`}
          isUnrevealed={true}
          variant="row"
        />
      ))}
    </div>
  );
}

export default function PlayerStateViewer({
  battleState,
  playerData,
  allyPlayerData,
  localPlayerId,
  rules,
}: PlayerStateViewerProps) {
  if (!battleState || !battleState.field?.sides?.length) {
    return <p className="empty-state">None</p>;
  }

  return (
    <div className="flex-col gap-s w-full">
      {battleState.field.sides.map((side, sideIdx) => {
        const sideTitle = side.name || `Side ${sideIdx + 1}`;
        const sideConditions = stateSelectors.sideConditions(battleState, sideIdx);
        const players = stateSelectors
          .sidePlayers(battleState, sideIdx)
          .slice()
          .sort((a, b) => (a.position ?? 0) - (b.position ?? 0) || a.id.localeCompare(b.id));
        const isSinglePlayer = players.length === 1;

        const playerItems = players.map((player) => {
          const isLocalPlayer = localPlayerId != null && player.id === localPlayerId;
          const targetPlayerData = isLocalPlayer
            ? playerData
            : allyPlayerData?.[player.id];
          const counts = getPlayerCounts(
            player,
            sideIdx,
            battleState,
            targetPlayerData,
          );
          return {
            player,
            isLocalPlayer,
            targetPlayerData,
            counts,
          };
        });

        const single = isSinglePlayer ? playerItems[0] : null;

        return (
          <Fragment key={sideIdx}>
            {sideIdx > 0 && <hr className={styles.sideDivider} />}
            <section className="flex-col gap-xs w-full">
              {single ? (
                <div className={`flex-row justify-between align-center gap-xs ${styles.sideHeader}`}>
                  {renderPlayerLabel(single.player, single.isLocalPlayer)}
                  <div className="flex-row align-center gap-xs flex-wrap justify-end">
                    {renderSideConditions(sideConditions)}
                    <span className={styles.aliveCount}>
                      {single.counts.aliveCount}/{single.counts.totalCount}
                    </span>
                  </div>
                </div>
              ) : (
                <div className={`flex-row justify-between align-center gap-xs ${styles.sideHeader}`}>
                  <span className={styles.sideTitle}>{sideTitle}</span>
                  {sideConditions.length > 0 && (
                    <div className="flex-row gap-xs flex-wrap">
                      {renderSideConditions(sideConditions)}
                    </div>
                  )}
                </div>
              )}

              {playerItems.length === 0 && <p className="empty-state">None</p>}

              <div className="flex-col gap-xs">
                {playerItems.map(({ player, isLocalPlayer, targetPlayerData, counts }) => {
                  return (
                    <div key={player.id} className="flex-col gap-xs">
                      {!isSinglePlayer && (
                        <div className="flex-row justify-between align-center gap-s">
                          {renderPlayerLabel(player, isLocalPlayer)}
                          <span className={styles.aliveCount}>
                            {counts.aliveCount}/{counts.totalCount}
                          </span>
                        </div>
                      )}

                      {counts.hasPrivateTeam && targetPlayerData
                        ? renderPrivatePlayerTeam(
                            targetPlayerData,
                            player,
                            sideIdx,
                            battleState,
                            rules,
                          )
                        : renderPublicPlayerTeam(
                            player,
                            sideIdx,
                            battleState,
                            rules,
                          )}
                    </div>
                  );
                })}
              </div>
            </section>
          </Fragment>
        );
      })}
    </div>
  );
}
