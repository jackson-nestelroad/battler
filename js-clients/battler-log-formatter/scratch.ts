import { LogCategory } from "./src/types.js";
import type { UiLogEntry, BattleState, UiMon } from "battler-state";

export function resolveMonContext(monRef: UiMon | undefined, state: BattleState | undefined, options: any): any {
  if (!monRef) return { text: "Mon" };
  let name = "Mon";
  if ("Active" in monRef && monRef.Active) {
    if (monRef.Active.name) name = monRef.Active.name;
  } else if ("Bench" in monRef && monRef.Bench) {
    if (monRef.Bench.name) name = monRef.Bench.name;
  } else if ("Party" in monRef && monRef.Party) {
    if (monRef.Party.name) name = monRef.Party.name;
  } else if ("Inactive" in monRef && monRef.Inactive) {
    if (monRef.Inactive.name) name = monRef.Inactive.name;
  }
  return { text: name };
}

function buildPattern(title: string, tags: string[], flags: string[]): string {
    const sortedTags = [...tags].sort();
    const sortedFlags = [...flags].sort();
    return [title, ...sortedTags, ...sortedFlags].join('|');
}

function generateCombinatorics(title: string, baseTags: string[], flags: string[]): string[] {
    const results: string[] = [];

    // 1. Partial Tag Combinations
    if (baseTags.length > 1) {
        // Find if we have 'from' or 'of'
        const fromTag = baseTags.find(t => t.startsWith('from:'));
        const ofTag = baseTags.find(t => t.startsWith('of:'));

        if (fromTag && ofTag) {
            results.push(buildPattern(title, baseTags, flags));
            const fromOnly = baseTags.map(t => t.startsWith('of:') ? 'of:*' : t);
            results.push(buildPattern(title, fromOnly, flags));
        } else {
            results.push(buildPattern(title, baseTags, flags));
        }
    } else {
        results.push(buildPattern(title, baseTags, flags));
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
    const fullyGeneric = buildPattern(title, fullyGenericTags, flags);
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
        const noVarsPattern = buildPattern(pTitle, noVarsTags, pFlags);
        if (!finalResults.includes(noVarsPattern)) finalResults.push(noVarsPattern);
        
        // Omit from/of
        if (!requireFromOf.includes(pTitle)) {
            const noFromOfTags = pTags.filter(t => {
                const k = t.split(':')[0];
                return k !== 'from' && k !== 'of';
            });
            const noFromOfPattern = buildPattern(pTitle, noFromOfTags, pFlags);
            if (!finalResults.includes(noFromOfPattern)) finalResults.push(noFromOfPattern);
            
            // Omit both
            const noBothTags = noFromOfTags.filter(t => !pureVars.includes(t.split(':')[0]));
            const noBothPattern = buildPattern(pTitle, noBothTags, pFlags);
            if (!finalResults.includes(noBothPattern)) finalResults.push(noBothPattern);
        }
    }
    
    return finalResults;
}

export function mapUiLogEntry(entry: any, state?: BattleState, options: any = {}): any {
  if (typeof entry === 'string') {
      const lower = entry.toLowerCase();
      return { patterns: [lower], category: LogCategory.Primary, context: {} };
  }
  
  const keyStr = Object.keys(entry)[0];
  const key = keyStr.toLowerCase();
  const data = entry[keyStr];
  
  let title = key;
  
  // Hardcoded overrides
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

  if (data && data.title) {
      title = data.title;
  } else if (key === 'effect' && data && data.effect?.id) {
      title = data.effect.id;
  }

  const tags: string[] = [];
  const flags: string[] = [];
  const context: any = {};
  
  let category = LogCategory.Primary;
  
  // Custom categorization
  if (['debug', 'waiting', 'experience', 'moveupdate', 'addvolatile'].includes(key)) {
      category = LogCategory.Hint;
  }

  if (data && typeof data === 'object') {
      const processValue = (k: string, v: any) => {
          if (v === undefined || v === null) return;
          if (k === 'title') return;
          if (k === 'animate') {
              if (v === false) flags.push('[noanim]');
              return;
          }
          if (k === 'animate_only') {
              if (v === true && title === 'move') title = 'animatemove';
              return;
          }

          if (k === 'mon' || k === 'target' || k === 'source') {
              tags.push(`${k}:*`);
              context[k.toUpperCase()] = resolveMonContext(v, state, options);
          } else if (['ability', 'item', 'effect', 'condition', 'weather', 'status', 'volatile'].includes(k)) {
              tags.push(`${k}:${typeof v === 'string' ? v : '*'}`);
          } else if (k === 'move_name') {
              tags.push(`move:${v}`);
          } else if (k === 'move' && title !== 'move') {
              tags.push(`move:${typeof v === 'string' ? v : '*'}`);
          } else if (k === 'name' && title === 'move') {
              tags.push(`move:${v}`);
              context.MOVE = v;
          } else if (k === 'from') {
              if (typeof v === 'object' && v.effect_type && v.name) {
                  tags.push(`from:${v.effect_type}:${v.name}`);
              } else {
                  tags.push(`from:*`);
              }
          } else if (k === 'of' || k === 'player' || k === 'position') {
              tags.push(`${k}:*`);
          } else if (typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean') {
             if (k === 'max' && v === true) flags.push('[max]');
             else tags.push(`${k}:${v}`);
          } else {
             tags.push(`${k}:*`);
          }
      };

      for (const [k, v] of Object.entries(data)) {
          if (k === 'effect' && title !== 'effect') {
             if (typeof v === 'object' && v !== null) {
                 for (const [ek, ev] of Object.entries(v)) {
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
      } else if (title === 'switchout') {
          tags.length = 0;
          tags.push('mon:*');
      } else if (title === 'addedtype') {
          tags.length = 0;
          if (data.mon) tags.push('mon:*');
          if (data.type) tags.push('type:*');
      }
  }

  const patterns = generateCombinatorics(title, tags, flags);
  return { patterns, category, context };
}

console.log(JSON.stringify(mapUiLogEntry({"Move":{"mon":{"Active":{"side":0,"position":0,"player":"player-1","name":"Pidgeot"}},"name":"Fly","animate":false,"animate_only":false}}), null, 2));

