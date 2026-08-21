const DOCUMENTED_TITLES = new Set([
  "info", "side", "player", "teamsize", "teampreviewstart", "mon", "teampreview",
  "battlestart", "turn", "time", "continue", "residual", "turnlimit", "maxsidelength", 
  "win", "tie", "split", "switch", "drag", "appear", "switchout", "specieschange", "replace", "formechange",
  "mega", "revertmega", "primal", "revertprimal", "ultra", "revertultra", "gigantamax",
  "revertgigantamax", "dynamax", "revertdynamax", "tera", "reverttera", "cant", "fail",
  "immune", "miss", "supereffective", "resisted", "crit", "ohko", "faint", "waiting",
  "damage", "heal", "sethp", "revive", "curestatus", "addvolatile", "removevolatile",
  "addsidecondition", "removesidecondition", "swapsideconditions", "swapsidecondition",
  "addslotcondition", "removeslotcondition", "addpseudoweather", "removepseudoweather",
  "typechange", "resettypechange", "addedtype", "transform", "ability", "abilityend",
  "abilitystart", "item", "itemend", "itemstart", "move", "animatemove", "prepare", "hitcount", "boost", "unboost",
  "swapboosts", "invertboosts", "copyboosts", "clearboosts", 
  "clearnegativeboosts", "clearpositiveboosts", "exp", "levelup", "cannotescape", 
  "escaped", "forfeited", "useitem", "uncatchable", "catchfailed", "catch", "swap", 
  "swapplayer", "catchrate", "learnedmove", "didnotlearnmove", "fxlang_debug", "debug",
  "activate", "block", "deductpp", "end", "fieldactivate", "fieldend", "fieldstart", 
  "protectweaken", "restorepp", "setpp", "sideend", "sidestart", "singlemove", 
  "singleturn", "start", "status", "weather"
]);

export function maskLog(line: string): string | null {
  line = line.trim();
  if (!line) return null;
  
  const parts = line.split('|').map(p => p.trim());
  let title = parts[0];
  
  if (!DOCUMENTED_TITLES.has(title)) {
    return null;
  }
  
  // Workaround for unboost by:0 which maps to boost
  if (title === 'unboost' && parts.some(p => p.trim() === 'by:0')) {
    title = 'boost';
  }
  const maskedParts = [title];
  
  const excludeTags = [
      'mon', 'of', 'player', 'side', 'slot', 'position', 'source',
      'gender', 'health', 'level', 'name', 'species',
      'stats', 'stat', 'by', 'exp', 'atk', 'def', 'spa', 'spd', 'spe', 'hp',
      'target', 'anim', 'newmove', 'pick', 'size', 'length', 'id', 'time'
  ];

  // Extract all tag parts
  const tags: string[] = [];
  const flags: string[] = [];
  
  for (let i = 1; i < parts.length; i++) {
    const p = parts[i];
    if (!p) continue;
    
    if (p.includes(':')) {
      const splitIndex = p.indexOf(':');
      let k = p.substring(0, splitIndex);
      const v = p.substring(splitIndex + 1);
      
      if (k === 'into_position') k = 'position';
      if (k === 'item' && title === 'useitem') k = 'name';
      
      if (excludeTags.includes(k)) continue;
      
      let keepSpecific = false;
      if (k === 'from') {
          keepSpecific = true;
      } else if (['ability', 'item', 'move', 'effect', 'condition', 'weather', 'status', 'volatile'].includes(k)) {
          keepSpecific = true;
      }
      
      // Forced genericizations based on title
      if (title === 'abilitystart' || title === 'itemstart' || title === 'abilityend') {
          if (k === 'ability' || k === 'item') keepSpecific = false;
      }
      if (title === 'addvolatile' && k === 'volatile') keepSpecific = false;
      if (title === 'block' && ['ability', 'move', 'item'].includes(k)) keepSpecific = false;
      if (title === 'activate' && k === 'move') {
          const hasPrimary = parts.some(part => {
              const p = part.trim();
              return p.startsWith('ability:') || p.startsWith('item:') || p.startsWith('hit:') || p.startsWith('magnitude:');
          });
          if (hasPrimary) keepSpecific = false;
      }
      
      // Strip unrepresentable tags that the Rust engine intentionally drops from the UI log
      if (title === 'move' && (k === 'from' || k === 'spread' || k === '[zpower]' || k === '[noanim]' || k === '[notarget]')) continue;
      if (title === 'switchout' && (k === '[copysubstitute]' || k === '[copyvolatile]')) continue;
      if (title === 'switch' && k === 'tera') continue;

      if (keepSpecific) {
        tags.push(`${k}:${v}`);
      } else {
        tags.push(`${k}:*`);
      }
    } else {
      let flag = `[${p}]`;
      if (title === 'move' && (flag === '[zpower]' || flag === '[notarget]' || flag === '[miss]')) continue;
      if (title === 'switchout' && (flag === '[copysubstitute]' || flag === '[copyvolatile]')) continue;
      flags.push(flag);
    }
  }

  tags.sort();
  flags.sort();
  
  if (tags.length > 0) {
    maskedParts.push(...tags);
  }
  if (flags.length > 0) {
    maskedParts.push(...flags);
  }
  
  return maskedParts.join('|');
}

export function getExpectedEnumKey(line: string): string | null {
  const pattern = maskLog(line);
  if (!pattern) return null;
  
  const safePattern = pattern
    .replace(/\|/g, '__')
    .replace(/:/g, '_')
    .replace(/\*/g, 'any')
    .toLowerCase()
    .replace(/[^a-z0-9_]/g, '');
  return safePattern;
}
