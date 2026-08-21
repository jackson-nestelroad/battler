import type { UiLogEntry } from "battler-state";

export function getLogPatterns(entry: UiLogEntry): string[] {
  if (typeof entry === 'string') return [entry.toLowerCase()];
  const key = Object.keys(entry)[0] as keyof UiLogEntry;
  const originalData = (entry as any)[key];
  const data = typeof originalData === 'object' && originalData !== null ? JSON.parse(JSON.stringify(originalData)) : originalData;
  let title = '';
  const tags: string[] = [];
  
const pushTag = (tag: string, source: string) => {
    // console.log(`Pushing ${tag} from ${source}`);
    tags.push(tag);
  };

  const flags: string[] = [];
  switch (key) {
        case 'AddedType':
      title = 'addedtype';
      if (data.mon !== undefined) tags.push('mon:*');
      if (data.type !== undefined) tags.push('type:*');
      break;
    case 'Damage':
      title = 'damage';
      if (data.health !== undefined) tags.push('health:*');
      break;
    case 'Debug':
      if (data.title) title = data.title;
      else title = 'debug';
      if (data.title !== undefined) tags.push('title:*');
      break;
    case 'Extension':
      title = 'extension';
      if (data.source !== undefined) tags.push('source:*');
      if (data.title !== undefined) tags.push('title:*');
      break;
    case 'Heal':
      title = 'heal';
      if (data.health !== undefined) tags.push('health:*');
      break;
    case 'Leave':
      if (data.title) title = data.title;
      else title = 'leave';
      if (data.player !== undefined) tags.push('player:*');
      delete data.positions;
      break;
    case 'LevelUp':
      title = 'levelup';
      if (data.mon !== undefined) tags.push('mon:*');
      if (data.level !== undefined) tags.push('level:*');
      if (data.stats !== undefined) tags.push('atk:*', 'def:*', 'hp:*', 'spa:*', 'spd:*', 'spe:*');
      break;
    case 'Move':
      title = data.animate_only ? 'animatemove' : 'move';
      break;
    case 'MoveUpdate':
      title = data.learned ? 'learnedmove' : 'didnotlearnmove';
      if (data.mon !== undefined) tags.push('mon:*');
      if (data.move_name !== undefined) tags.push(`move:${data.move_name}`);
      delete data.move_name;
      delete data.learned;
      break;
    case 'SetHealth':
      title = 'sethp';
      if (data.health !== undefined) tags.push('health:*');
      break;
    case 'StatBoost':
      title = data.by < 0 ? 'unboost' : 'boost';
      if (data.mon !== undefined) tags.push('mon:*');
      if (data.stat !== undefined) tags.push('stat:*');
      if (data.by !== undefined) tags.push('by:*');
      break;
    case 'Switch':
      if (data.title) title = data.title;
      else title = 'switch';
      
      // Since Switch strips all visual details (level, species, health), we just hardcode the known tag combinations from en.ts
      if (title === 'switch' || title === 'drag' || title === 'replace' || title === 'appear') {
          tags.push('player:*');
          tags.push('position:*');
          tags.push('name:*');
          tags.push('health:*');
          tags.push('species:*');
          tags.push('level:*');
          tags.push('gender:*');
          if (title === 'switch' || title === 'drag') {
              // Note: en.ts might have tera:* for switch, but permutations won't catch it. 
              // We'll just push tera:* as a possible tag.
              // Actually, we can return multiple patterns but getLogPattern returns one.
          }
      } else if (title === 'switchout') {
          tags.push('mon:*');
      }
      break;
    case 'UpdateAppearance':
      if (data.title) title = data.title;
      else title = 'updateappearance';
      
      // UpdateAppearance strips health, status, level, gender
      if (title === 'specieschange') {
          tags.push('player:*');
          tags.push('position:*');
          tags.push('name:*');
          tags.push('health:*');
          tags.push('species:*');
          tags.push('level:*');
          tags.push('gender:*');
      } else if (title === 'formechange') {
          tags.push('mon:*');
          tags.push('species:*');
      }
      break;
        case 'Experience':
      title = 'exp';
      break;
    case 'SwitchOut':
      title = 'switchout';
      if (data.mon !== undefined) tags.push('mon:*');
      break;
    case 'UseItem':
      title = 'useitem';
      if (data.player !== undefined) tags.push('player:*');
      if (data.item !== undefined) tags.push('name:*');
      if (data.target !== undefined) tags.push('target:*');
      break;
    default:
      if (data && data.title) {
         title = data.title;
      } else if (key === 'Effect' && data.effect?.id) {
         title = data.effect.id;
      } else {
         title = (key as string).toLowerCase();
         if (title === 'caught') title = 'catch';
         else if (title === 'fainted') title = 'faint';
         else if (title === 'statdrop') title = 'unboost';
         else if (title === 'itemend') title = 'itemend';
         else if (title === 'clearweather') title = 'clearweather';
         else if (title === 'fieldstart') title = 'fieldstart';
         else if (title === 'fieldend') title = 'fieldend';
         else if (title === 'sidestart') title = 'sidestart';
         else if (title === 'sideend') title = 'sideend';
         else if (title === 'curestatus') title = 'curestatus';
      }
      
      break;
  }

  // Auto-reconstruct fields for all variants
  if (data) {
      for (const [k, v] of Object.entries(data)) {
        if (k === 'effect' || k === 'title' || k === 'animate' || k === 'animate_only' || k === 'stats') continue;
        if (k === 'health' && (title === 'damage' || title === 'heal' || title === 'sethp' || title === 'appear')) continue;
        
        const keyName = k === 'into_position' ? 'position' : (k === 'item' && title === 'useitem' ? 'name' : k);
        if (keyName === 'mon' && ['switch', 'replace', 'drag', 'appear', 'switchout'].includes(title)) continue;
        // Prevent duplicate tags
        if (tags.some(t => t.startsWith(`${keyName}:`))) continue;
        
        if (v !== undefined && v !== null) {
            if (['ability', 'item', 'move', 'effect', 'condition', 'weather', 'status', 'volatile', 'from'].includes(keyName) && typeof v === 'string') {
                tags.push(`${keyName}:${v}`);
            } else {
                tags.push(`${keyName}:*`);
            }
        }
     }
  }

  if (data?.effect) {
    if (data.effect.target !== undefined) {
        if (!tags.some(t => t.startsWith('mon:'))) tags.push('mon:*');
    }
    if (data.effect.source !== undefined) {
        tags.push('of:*');
    }
    if (data.effect.player !== undefined) {
        tags.push('player:*');
    }
    if (data.effect.side !== undefined) {
        tags.push('side:*');
    }
    if (data.effect.slot !== undefined) {
        tags.push('slot:*');
    }
    if (data.effect.source_effect !== undefined) {
        if (typeof data.effect.source_effect === 'string') {
           tags.push(`from:${data.effect.source_effect}:*`);
        } else if (data.effect.source_effect.effect_type) {
           const typeStr = data.effect.source_effect.effect_type.toLowerCase();
           const valStr = data.effect.source_effect.name || '*';
           tags.push(`from:${typeStr}:${valStr}`);
        } else if (data.effect.source_effect.name) {
           tags.push(`from:${data.effect.source_effect.name}`);
        } else {
           tags.push(`from:*`);
        }
    }
    if (data.effect.effect !== undefined) {
        let typeStr = '';
        let valStr = '*';
        if (typeof data.effect.effect === 'string') {
           typeStr = 'effect';
           valStr = data.effect.effect;
        } else if (data.effect.effect.effect_type) {
           typeStr = data.effect.effect.effect_type.toLowerCase();
           if (['ability', 'item', 'move', 'effect', 'condition', 'weather', 'status', 'volatile'].includes(typeStr)) {
               valStr = data.effect.effect.name;
           }
        }
        if (typeStr) tags.push(`${typeStr}:${valStr}`);
    }

    if (data.effect.additional !== undefined) {
      for (const [k, v] of Object.entries(data.effect.additional as Record<string, string>)) {
        if (k === 'health' && (title === 'damage' || title === 'heal' || title === 'sethp' || title === 'appear')) continue;
        if (v === "") flags.push(`[${k}]`);
        else if (['ability', 'item', 'move', 'effect', 'condition', 'weather', 'status', 'volatile'].includes(k)) tags.push(`${k}:${v}`);
        else tags.push(`${k}:*`);
      }
    }
  }

  const excludeTags = [
      'mon', 'of', 'player', 'side', 'slot', 'position', 'source',
      'gender', 'health', 'level', 'name', 'species',
      'stats', 'stat', 'by', 'exp', 'atk', 'def', 'spa', 'spd', 'spe', 'hp',
      'target', 'anim', 'newmove', 'pick', 'size', 'length', 'id', 'time'
  ];

  let baseTags = Array.from(new Set(tags))
      .filter(t => !excludeTags.some(ex => t.startsWith(`${ex}:`)))
      .sort();
  flags.sort();

  // For specific logs, force specific attributes to ANY
  if (title === 'abilitystart' || title === 'itemstart' || title === 'abilityend') {
      baseTags = baseTags.map(t => {
          if (t.startsWith('ability:') && t !== 'ability:*') return 'ability:*';
          if (t.startsWith('item:') && t !== 'item:*') return 'item:*';
          return t;
      });
  } else if (title === 'addvolatile') {
      baseTags = baseTags.map(t => {
          if (t.startsWith('volatile:') && t !== 'volatile:*') return 'volatile:*';
          return t;
      });
  } else if (title === 'block') {
      baseTags = baseTags.map(t => {
          if (t.startsWith('ability:') && t !== 'ability:*') return 'ability:*';
          if (t.startsWith('move:') && t !== 'move:*') return 'move:*';
          if (t.startsWith('item:') && t !== 'item:*') return 'item:*';
          return t;
      });
  } else if (title === 'activate') {
      const hasPrimary = baseTags.some(t => t.startsWith('ability:') || t.startsWith('item:') || t.startsWith('hit:') || t.startsWith('magnitude:'));
      if (hasPrimary) {
          baseTags = baseTags.map(t => {
              if (t.startsWith('move:') && t !== 'move:*') return 'move:*';
              return t;
          });
      }
  }

  const buildPattern = (t: string[]) => [title, ...Array.from(new Set(t)).sort(), ...flags].join('|');
  const results: string[] = [];

  // 1. Core Permutations
  if (title === 'ability' || title === 'item') {
      const hasName = baseTags.some(t => (t.startsWith(`${title}:`) && t !== `${title}:*`));
      const hasFrom = baseTags.some(t => t.startsWith('from:') && !t.endsWith(':*'));
      
      if (hasName && hasFrom) {
          // Exact match (Both)
          results.push(buildPattern(baseTags));
          
          // Name only (from genericized)
          const nameOnly = baseTags.map(t => {
              if (t.startsWith('from:') && !t.endsWith(':*')) {
                  const parts = t.split(':');
                  return `${parts[0]}:${parts[1]}:*`;
              }
              return t;
          });
          results.push(buildPattern(nameOnly));
          
          // From only (name genericized)
          const fromOnly = baseTags.map(t => {
              if (t.startsWith(`${title}:`) && t !== `${title}:*`) {
                  return `${title}:*`;
              }
              return t;
          });
          results.push(buildPattern(fromOnly));
      } else {
          results.push(buildPattern(baseTags));
      }
  } else {
      results.push(buildPattern(baseTags));
  }

  // 2. Fully Generic
  const fullyGenericTags = baseTags.map(t => {
      const parts = t.split(':');
      if (parts.length === 2 && ['ability', 'item', 'move', 'effect', 'condition', 'weather', 'status', 'volatile'].includes(parts[0]) && parts[1] !== '*') {
          return `${parts[0]}:*`;
      }
      if (parts.length === 3 && parts[0] === 'from' && parts[2] !== '*') {
          return `${parts[0]}:${parts[1]}:*`;
      }
      return t;
  });
  const fullyGeneric = buildPattern(fullyGenericTags);
  if (!results.includes(fullyGeneric)) results.push(fullyGeneric);

  // 3. Apply Modifiers (pure vars & from/of)
  const requireFromOf = ['damage', 'heal', 'sethp', 'item', 'itemend', 'itemstart', 'ability', 'abilitystart', 'cant', 'fail', 'immune', 'block'];
  const pureVars = ['by', 'exp', 'level', 'hp', 'atk', 'def', 'spa', 'spd', 'spe', 'stats', 'stat'];
  
  const finalResults: string[] = [];
  for (const pattern of results) {
      if (!finalResults.includes(pattern)) finalResults.push(pattern);
      
      const pParts = pattern.split('|');
      const pTitle = pParts[0];
      const pTags = pParts.slice(1).filter(x => !x.startsWith('[') && !x.endsWith(']'));
      const pFlags = pParts.slice(1).filter(x => x.startsWith('[') && x.endsWith(']'));
      
      // Omit pure vars
      const noVarsTags = pTags.filter(t => !pureVars.includes(t.split(':')[0]));
      const noVarsPattern = [pTitle, ...noVarsTags, ...pFlags].join('|');
      if (!finalResults.includes(noVarsPattern)) finalResults.push(noVarsPattern);
      
      // Omit from/of
      if (!requireFromOf.includes(pTitle)) {
          const noFromOfTags = pTags.filter(t => {
              const k = t.split(':')[0];
              return k !== 'from' && k !== 'of';
          });
          const noFromOfPattern = [pTitle, ...noFromOfTags, ...pFlags].join('|');
          if (!finalResults.includes(noFromOfPattern)) finalResults.push(noFromOfPattern);
          
          // Omit both
          const noBothTags = noFromOfTags.filter(t => !pureVars.includes(t.split(':')[0]));
          const noBothPattern = [pTitle, ...noBothTags, ...pFlags].join('|');
          if (!finalResults.includes(noBothPattern)) finalResults.push(noBothPattern);
      }
  }
  
  return finalResults;
}
