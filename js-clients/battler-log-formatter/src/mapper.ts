
import type { UiLogEntry, BattleState, UiMon } from "battler-state";
import i18next from "i18next";
import { LogCategory } from "./types.js";
import type { MapperOptions, AnyMappedLog, ContextVar } from "./types.js";

function getPlayerName(state: BattleState | undefined, playerId: string | undefined): string {
  if (!playerId) return "";
  if (state?.field?.sides) {
    for (const side of state.field.sides) {
      if (side?.players?.[playerId]) {
        return side.players[playerId].name;
      }
    }
  }
  return playerId;
}

function getSideName(state: BattleState | undefined, sideIndex: number | undefined): string {
  if (sideIndex == null) return "";
  if (state?.field?.sides?.[sideIndex]) {
    return state.field.sides[sideIndex].name;
  }
  return `Player ${sideIndex}`;
}

export function resolveMonContext(monRef: UiMon | undefined, state: BattleState | undefined, options: MapperOptions): ContextVar {
  if (!monRef || typeof monRef !== 'object') return { text: "Mon" };
  
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

  const isAlly = options.localPlayerId === playerId;
  const playerName = getPlayerName(state, playerId) || playerId;
  let text = name;
  let noAutoCapitalize = false;
  
  if (playerId && !isAlly) {
    if (options.foeFormat === "possessive") {
      text = i18next.t("mon.foe_possessive", { name, player: playerName });
      noAutoCapitalize = true;
    } else if (options.foeFormat === "withPlayer") {
      text = i18next.t("mon.foe_with_player", { name, player: playerName });
    } else {
      text = i18next.t("mon.foe", { name });
    }
  } else {
    if (options.allyFormat === "possessive") {
      text = i18next.t("mon.ally_possessive", { name });
    } else {
      text = i18next.t("mon.ally", { name });
    }
  }

  return { text, id, noAutoCapitalize };
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
        
        recurse(index + 1, [...currentTags, tag]);
        
        if (parts.length >= 2) {
            const k = parts[0];
            if (['ability', 'item', 'move', 'effect', 'condition', 'weather', 'status', 'volatile', 'type', 'clause', 'species'].includes(k) && parts[1] !== '*') {
                recurse(index + 1, [...currentTags, `${k}:*`]);
            }
            if (k === 'from' && parts.length === 3 && parts[2] !== '*') {
                recurse(index + 1, [...currentTags, `${parts[0]}:${parts[1]}:*`]);
            }
        }
    }
    
    recurse(0, []);
    
    const finalResults = Array.from(results);
    const requireFromOf = ['damage', 'heal', 'sethp', 'item', 'itemend', 'itemstart', 'ability', 'abilitystart', 'cant', 'fail', 'immune', 'block'];
    
    if (!requireFromOf.includes(title)) {
        for (const pattern of Array.from(results)) {
            const pParts = pattern.split('|');
            const pTags = pParts.slice(1).filter(x => !x.startsWith('[') && !x.endsWith(']'));
            const pFlags = pParts.slice(1).filter(x => x.startsWith('[') && x.endsWith(']'));
            
            const noFromOfTags = pTags.filter(t => {
                const k = t.split(':')[0];
                return k !== 'from' && k !== 'of';
            });
            finalResults.push(buildPattern(title, noFromOfTags, pFlags));
        }
    }
    
    return Array.from(new Set(finalResults)).sort((a, b) => b.length - a.length);
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
  
  let category = LogCategory.Primary;
  if (['debug', 'waiting', 'experience', 'moveupdate', 'addvolatile'].includes(key)) {
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
              if (v === false) flags.push('[noanim]');
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
              context[k.toUpperCase()] = resolveMonContext(v, state, options);
          } else if (k === 'target') {
              if (key === 'effect') {
                  tags.push(`mon:*`);
                  context.MON = resolveMonContext(v, state, options);
              } else {
                  tags.push(`target:*`);
                  context.TARGET = resolveMonContext(v, state, options);
              }
          } else if (k === 'source') {
              tags.push(`of:*`);
              context.SOURCE = resolveMonContext(v, state, options);
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
              if (k === 'player') context.PLAYER = { text: getPlayerName(state, v), noAutoCapitalize: true };
          } else if (typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean') {
             if (v === true || v === "") {
                 flags.push(`[${k}]`);
             } else {
                 if (['ability', 'item', 'move', 'effect', 'condition', 'weather', 'status', 'volatile', 'from'].includes(k)) {
                     tags.push(`${k}:${v}`);
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
          context.PLAYER = { text: getPlayerName(state, data.player), noAutoCapitalize: true };
          context.MON = { text: data.name || "Mon" };
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
      'stats', 'stat', 'by', 'exp', 'atk', 'def', 'spa', 'spd', 'spe', 'hp',
      'target', 'anim', 'newmove', 'pick', 'size', 'length', 'id', 'time'
  ];

  const combinatoricTags = tags.filter(t => !excludeTags.includes(t.split(':')[0]));

  const patterns = generateCombinatorics(title, combinatoricTags, flags);
  
  const mapped: AnyMappedLog = {
      patterns,
      category,
      context,
      ...(data.effect ? { effect: data.effect } : {})
  };
  return mapped;
}

export function getLogPatterns(entry: UiLogEntry): string[] {
    return mapUiLogEntry(entry)?.patterns || [];
}
