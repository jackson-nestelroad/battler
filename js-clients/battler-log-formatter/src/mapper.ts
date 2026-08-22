
import type { UiLogEntry, BattleState, UiMon } from "battler-state";
import i18next from "i18next";
import { LogCategory } from "./types.js";
import type { MapperOptions, AnyMappedLog, ContextVar } from "./types.js";


export type Relationship = "self" | "ally" | "foe";

export function getRelationship(state: BattleState | undefined, localPlayerId: string | undefined, targetPlayerId: string): Relationship {
  if (!localPlayerId) return "foe";
  if (localPlayerId === targetPlayerId) return "self";
  
  if (state?.field?.sides) {
    let localSideIndex = -1;
    let targetSideIndex = -1;
    
    for (let i = 0; i < state.field.sides.length; i++) {
      const side = state.field.sides[i];
      if (side?.players?.[localPlayerId]) localSideIndex = i;
      if (side?.players?.[targetPlayerId]) targetSideIndex = i;
    }
    
    if (localSideIndex !== -1 && localSideIndex === targetSideIndex) {
      return "ally";
    }
  }
  
  return "foe";
}

export function getSideRelationship(state: BattleState | undefined, localPlayerId: string | undefined, sideIndex: number): Relationship {
    if (!localPlayerId) return "foe";
    if (state?.field?.sides?.[sideIndex]?.players?.[localPlayerId]) {
        return "self";
    }
    return "foe";
}

export function resolvePlayerContext(playerId: string | undefined, state: BattleState | undefined, options: MapperOptions): { standard: ContextVar, possessive: ContextVar } {
    let name = playerId || "Player";
    if (playerId && state?.field?.sides) {
        for (const side of state.field.sides) {
            if (side?.players?.[playerId]) {
                name = side.players[playerId].name;
                break;
            }
        }
    }

    if (!playerId) {
        return {
            standard: { text: name },
            possessive: { text: `${name}'s` }
        };
    }

    const rel = getRelationship(state, options.localPlayerId, playerId);
    
    let text = "";
    let possessiveText = "";
    
    if (rel === "self") {
        text = i18next.t("player.self");
        possessiveText = i18next.t("player.self_possessive");
    } else if (rel === "ally") {
        text = i18next.t("player.ally", { player: name });
        possessiveText = i18next.t("player.ally_possessive", { player: name });
    } else {
        text = i18next.t("player.foe", { player: name });
        possessiveText = i18next.t("player.foe_possessive", { player: name });
    }

    return {
        standard: { text, noAutoCapitalize: rel === "self" },
        possessive: { text: possessiveText, noAutoCapitalize: rel === "self" }
    };
}

export function resolveSideContext(sideIndex: number | undefined, state: BattleState | undefined, options: MapperOptions): { standard: ContextVar, possessive: ContextVar } {
    if (sideIndex == null) return { standard: { text: "Side" }, possessive: { text: "Side's" } };
    
    let name = `Player ${sideIndex}`;
    if (state?.field?.sides?.[sideIndex]) {
        name = state.field.sides[sideIndex].name;
    }

    const rel = getSideRelationship(state, options.localPlayerId, sideIndex);
    
    let text = "";
    let possessiveText = "";
    
    if (rel === "self") {
        text = i18next.t("side.self");
        possessiveText = i18next.t("side.self_possessive");
    } else {
        text = i18next.t("side.foe");
        possessiveText = i18next.t("side.foe_possessive");
    }

    return {
        standard: { text, noAutoCapitalize: true },
        possessive: { text: possessiveText, noAutoCapitalize: true }
    };
}

export function resolveMonContext(monRef: UiMon | undefined, state: BattleState | undefined, options: MapperOptions): { standard: ContextVar, possessive: ContextVar, raw: string, raw_possessive: string, ref?: UiMon } {
  if (!monRef || typeof monRef !== 'object') {
      return { standard: { text: "Mon" }, possessive: { text: "Mon's" }, raw: "Mon", raw_possessive: "Mon's" };
  }
  
  let name = "Mon";
  let playerId = "";
  let id = "";

  if ("Active" in monRef && monRef.Active) {
    if (monRef.Active.name) name = monRef.Active.name;
    if (monRef.Active.player) playerId = monRef.Active.player;
    id = `${playerId}-active-${monRef.Active.position}`;
  } else if ("Bench" in monRef && monRef.Bench) {
    if (monRef.Bench.name) name = monRef.Bench.name;
    if (monRef.Bench.player) playerId = monRef.Bench.player;
    id = `${playerId}-bench-${monRef.Bench.position}`;
  } else if ("Party" in monRef && monRef.Party) {
    if (monRef.Party.name) name = monRef.Party.name;
    if (monRef.Party.player) playerId = monRef.Party.player;
    id = `${playerId}-party-${monRef.Party.index}`;
  } else if ("Inactive" in monRef && monRef.Inactive) {
    if (monRef.Inactive.name) name = monRef.Inactive.name;
    if (monRef.Inactive.player) playerId = monRef.Inactive.player;
  }

  const rel = playerId ? getRelationship(state, options.localPlayerId, playerId) : "foe";
  const playerName = resolvePlayerContext(playerId, state, options).standard.text;
  
  let text = name;
  let possessiveText = `${name}'s`;
  let noAutoCapitalize = false;
  let possessiveNoAutoCapitalize = false;
  
  if (rel === "self") {
      text = i18next.t("mon.self", { name });
      possessiveText = i18next.t("mon.self_possessive", { name });
  } else if (rel === "ally") {
      text = i18next.t("mon.ally", { name });
      possessiveText = i18next.t("mon.ally_possessive", { name });
  } else {
      const isMulti = state?.settings?.battle_type === "Multi";
      if (isMulti) {
          text = i18next.t("mon.foe_multi", { name, player: playerName });
          possessiveText = i18next.t("mon.foe_possessive_multi", { name, player: playerName });
          noAutoCapitalize = true;
          possessiveNoAutoCapitalize = true;
      } else {
          text = i18next.t("mon.foe_single", { name });
          possessiveText = i18next.t("mon.foe_possessive_single", { name });
      }
  }

  return { 
      standard: { text, monRef, noAutoCapitalize },
      possessive: { text: possessiveText, monRef, noAutoCapitalize: possessiveNoAutoCapitalize },
      raw: name,
      raw_possessive: `${name}'s`,
      ref: monRef,
      rel
  };
}
function buildPattern(title: string, tags: string[], flags: string[]): string {
    const sortedTags = [...tags].sort();
    const sortedFlags = [...flags].sort();
    return [title, ...sortedTags, ...sortedFlags].join('|');
}

export function generateCombinatorics(title: string, baseTags: string[], flags: string[]): string[] {
    const results: Set<string> = new Set();
    
    function recurse(index: number, currentTags: string[]) {
        if (index === baseTags.length) {
            results.add(buildPattern(title, currentTags, flags));
            return;
        }
        
        const tag = baseTags[index];
        const parts = tag.split(':');
        const k = parts[0];
        
        // 1. Include specific tag
        recurse(index + 1, [...currentTags, tag]);
        
        // 2. Include generic tag (if applicable)
        if (parts.length >= 2) {
            if (['ability', 'item', 'move', 'effect', 'condition', 'weather', 'status', 'volatile', 'type', 'clause', 'species'].includes(k) && parts[1] !== '*') {
                recurse(index + 1, [...currentTags, `${k}:*`]);
            }
            if (k === 'from' && parts.length === 3 && parts[2] !== '*') {
                recurse(index + 1, [...currentTags, `${parts[0]}:${parts[1]}:*`]);
            }
            
            // 3. Omit tag entirely
            if (['from', 'of', 'by', 'source', 'battletype'].includes(k)) {
                recurse(index + 1, currentTags);
            }
        }
    }
    
    recurse(0, []);
    
    return Array.from(results).sort((a, b) => b.length - a.length);
}


export function mapUiLogEntry(entry: UiLogEntry, state?: BattleState, options: MapperOptions = {}): AnyMappedLog | null {
  if (typeof entry === 'string') {
      const lower = entry.toLowerCase();
      let context = {};
      return { patterns: [lower], category: LogCategory.Primary, context };
  }
  
  const keyStr = Object.keys(entry)[0] as keyof UiLogEntry;
  const key = keyStr.toLowerCase();
  const data = (entry as Record<string, any>)[keyStr];
  
  let title = key;
  let metadata: any = {};
  
  if (title === 'caught') title = 'catch';
  else if (title === 'fainted') title = 'faint';
  else if (title === 'statboost') title = data?.by < 0 ? 'unboost' : 'boost';
  else if (title === 'itemend') title = 'itemend';
  else if (title === 'experience') title = 'exp';
  else if (title === 'sethealth') title = 'sethp';
  else if (title === 'clearweather') title = 'clearweather';
  else if (title === 'fieldstart') title = 'fieldstart';
  else if (title === 'fieldend') title = 'fieldend';
  else if (title === 'sidestart') title = 'sidestart';
  else if (title === 'sideend') title = 'sideend';
  else if (title === 'curestatus') title = 'curestatus';
  else if (title === 'updateappearance') {
      if (data && data.title === 'specieschange') title = 'specieschange';
  } else if (title === 'effect') {
      if (data && data.effect?.id) title = data.effect.id;
  }

  if (data && data.title) {
      title = data.title;
  }

  const tags: string[] = [];
  const flags: string[] = [];
  const context: Record<string, ContextVar> = {};
  
  const PRIMARY_KEYS = [
      'move', 'switch', 'drag', 'faint', 
      'mega', 'primal', 'burst', 'zpower', 'dynamax', 'gigantamax', 'tera', 'terastallize', 
      'turn', 'win', 'tie', 'useitem', 'residual'
  ];

  const HINT_KEYS = [
      'addvolatile', 'animatemove', 'battlestart', 'debug', 'info', 
      'maxsidelength', 'mon', 'player', 'removevolatile', 'split', 
      'teampreview', 'teampreviewstart', 'teamsize', 'time'
  ];

  let category = LogCategory.Secondary;
  if (PRIMARY_KEYS.includes(key) || PRIMARY_KEYS.includes(title)) {
      category = LogCategory.Primary;
  } else if (HINT_KEYS.includes(key) || HINT_KEYS.includes(title)) {
      category = LogCategory.Hint;
  }

  if (data && typeof data === 'object') {
      const processValue = (k: string, v: any) => {
          if (v === undefined || v === null) return;
          if (k === 'title') return;
          
          if (k === 'additional' && typeof v === 'object' && v !== null) {
              for (const [ak, av] of Object.entries(v)) {
                  processValue(ak, av);
              }
              return;
          }

          if (k === 'animate') {
              if (v === false) flags.push('noanim');
              return;
          }
          if (k === 'animate_only') {
              if (v === true && title === 'move') title = 'animatemove';
              return;
          }
          
          if (k === 'learned' && key === 'moveupdate') {
              title = v ? 'learnedmove' : 'didnotlearnmove';
              return;
          }

          if (k === 'mon') {
              tags.push(`${k}:*`);
              const resolved = resolveMonContext(v, state, options);
              context[k.toUpperCase()] = resolved.standard;
              context[`${k.toUpperCase()}_POSSESSIVE`] = resolved.possessive;
              context.FOE_SIDE = (resolved.rel === "self" || resolved.rel === "ally") ? i18next.t("side.foe") : i18next.t("side.self");
          } else if (k === 'target') {
              if (key === 'effect') {
                  tags.push(`mon:*`);
                  const resolved = resolveMonContext(v, state, options);
                  context.MON = resolved.standard;
                  context.MON_POSSESSIVE = resolved.possessive;
                  context.FOE_SIDE = (resolved.rel === "self" || resolved.rel === "ally") ? i18next.t("side.foe") : i18next.t("side.self");
                  metadata.mon = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
              } else {
                  tags.push(`target:*`);
                  const resolved = resolveMonContext(v, state, options);
                  context.TARGET = resolved.standard;
                  context.TARGET_POSSESSIVE = resolved.possessive;
                  context.FOE_SIDE = (resolved.rel === "self" || resolved.rel === "ally") ? i18next.t("side.foe") : i18next.t("side.self");
                  metadata.target = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
              }
          } else if (k === 'source' || k === 'of') {
              tags.push(`of:*`);
              const resolved = resolveMonContext(v, state, options);
              context.SOURCE = resolved.standard;
              context.SOURCE_POSSESSIVE = resolved.possessive;
              context.FOE_SIDE = (resolved.rel === "self" || resolved.rel === "ally") ? i18next.t("side.foe") : i18next.t("side.self");
              metadata.source = { raw: resolved.raw, raw_possessive: resolved.raw_possessive, ref: resolved.ref };
          } else if (k === 'effect') {
              if (typeof v === 'object' && v !== null && v.name) {
                  if (v.effect_type) tags.push(`${v.effect_type}:${v.name}`);
                  else tags.push(`effect:${v.name}`);
              } else {
                  tags.push(`effect:${typeof v === 'string' ? v : '*'}`);
              }
          } else if (k === 'source_effect') {
              if (typeof v === 'object' && v !== null && v.name) {
                  if (v.effect_type) tags.push(`from:${v.effect_type}:${v.name}`);
                  else tags.push(`from:${v.name}`);
              } else {
                  tags.push(`from:*`);
              }
          } else if (k === 'move_name' && (title === 'learnedmove' || title === 'didnotlearnmove' || key === 'moveupdate')) {
              tags.push(`move:${v}`);
              context.MOVE = v;
          } else if (k === 'name' && title === 'move') {
              tags.push(`name:*`);
              context.MOVE = v;
          } else if (k === 'from') {
              if (typeof v === 'object' && v.effect_type && v.name) {
                  tags.push(`from:${v.effect_type}:${v.name}`);
              } else {
                  tags.push(`from:*`);
              }
          } else if (k === 'of' || k === 'player' || k === 'position') {
              tags.push(`${k}:*`);
              if (k === 'player') {
                  const resolved = resolvePlayerContext(v, state, options);
                  context.PLAYER = resolved.standard;
                  context.PLAYER_POSSESSIVE = resolved.possessive;
              }
          } else if (k === 'side') {
              tags.push(`side:*`);
              const resolved = resolveSideContext(Number(v), state, options);
              context.SIDE = resolved.standard;
              context.SIDE_POSSESSIVE = resolved.possessive;
          } else if (typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean') {
             if (v === true || v === "") {
                 flags.push(k);
             } else if (v === false) {
                 // Do not push false booleans to tags, they represent the absence of a flag
             } else {
                 if (['ability', 'item', 'move', 'effect', 'condition', 'weather', 'status', 'volatile', 'from'].includes(k)) {
                     tags.push(`${k}:${v}`);
                 } else if (k === 'by') {
                     const num = Number(v);
                     if (!isNaN(num) && num >= 3) tags.push('by:3plus');
                     else tags.push(`by:${v}`);
                 } else if (k === 'stat') {
                     context.STAT = i18next.t(`stats.${v}`);
                     tags.push(`${k}:*`);
                 } else {
                     tags.push(`${k}:*`);
                 }
             }
          } else {
             tags.push(`${k}:*`);
          }
      };

      for (const [k, v] of Object.entries(data)) {
          if (k === 'effect') {
             if (typeof v === 'object' && v !== null) {
                 for (const [ek, ev] of Object.entries(v as any)) {
                     processValue(ek, ev);
                 }
             }
          } else {
             processValue(k, v);
          }
      }
      
      if (['switch', 'drag', 'replace', 'appear'].includes(title)) {
          tags.length = 0;
          ['player', 'position', 'name', 'health', 'species', 'level', 'gender'].forEach(t => tags.push(`${t}:*`));
          const resolvedPlayer = resolvePlayerContext(data.player, state, options);
          context.PLAYER = resolvedPlayer.standard;
          context.PLAYER_POSSESSIVE = resolvedPlayer.possessive;
          context.MON = { text: data.name || "Mon" };
          context.MON_POSSESSIVE = { text: `${data.name || "Mon"}'s` };
          metadata.mon = { raw: data.name || "Mon", raw_possessive: `${data.name || "Mon"}'s` };
          if (typeof data.mon === 'object') {
              metadata.mon.ref = data.mon;
          }
      } else if (title === 'switchout') {
          tags.length = 0;
          tags.push('mon:*');
      }
  }

  if (title === 'leave' && data.title !== 'forfeited' && data.title !== 'escaped') return null;
  if (title === 'extension') return null;

  const excludeTags = [
      'mon', 'of', 'player', 'side', 'slot', 'position', 'positions', 'source',
      'gender', 'health', 'level', 'name', 'species',
      'stats', 'stat', 'exp', 'atk', 'def', 'spa', 'spd', 'spe', 'hp',
      'target', 'anim', 'newmove', 'pick', 'size', 'length', 'id', 'time'
  ];

  const combinatoricTags = tags.filter(t => !excludeTags.includes(t.split(':')[0]));
  
  if (state?.settings?.battle_type) {
      combinatoricTags.push(`battletype:${state.settings.battle_type.toLowerCase()}`);
  }

  let finalFlags = flags;
  let finalTags = combinatoricTags;
  
  if (title === 'move') {
      finalFlags = flags.filter(f => !['z_power', 'no_target'].includes(f));
      finalTags = combinatoricTags.filter(t => !t.startsWith('from:') && !t.startsWith('animate:') && !t.startsWith('animate_only:'));
  }

  const patterns = generateCombinatorics(title, finalTags, finalFlags);
  
  const mapped: AnyMappedLog = {
      patterns,
      category,
      context,
      effect: data?.effect ? (data.effect as EffectData) : undefined,
      metadata: Object.keys(metadata).length > 0 ? metadata : undefined
  };
  return mapped;
}

export function getLogPatterns(entry: UiLogEntry): string[] {
    return mapUiLogEntry(entry)?.patterns || [];
}
