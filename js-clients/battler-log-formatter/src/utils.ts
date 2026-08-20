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
  "item", "itemend", "move", "animatemove", "prepare", "hitcount", "boost", "unboost",
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
  const title = parts[0];
  
  if (!DOCUMENTED_TITLES.has(title)) {
    return null;
  }
  
  const maskedParts = [title];
  
  // Extract all tag parts
  const tags: string[] = [];
  const flags: string[] = [];
  
  for (let i = 1; i < parts.length; i++) {
    const p = parts[i];
    if (!p) continue;
    
    if (p.includes(':')) {
      const splitIndex = p.indexOf(':');
      const k = p.substring(0, splitIndex);
      const v = p.substring(splitIndex + 1);
      
      if (k === 'from' || k === 'of') {
        // e.g. from:ability:Intimidate -> from:ability:*
        if (v.includes(':')) {
          const type = v.substring(0, v.indexOf(':'));
          tags.push(`${k}:${type}:*`);
        } else {
          tags.push(`${k}:*`);
        }
      } else if (['ability', 'item', 'move', 'effect', 'condition', 'weather', 'status', 'volatile'].includes(k)) {
        tags.push(`${k}:${v}`);
      } else {
        tags.push(`${k}:*`);
      }
    } else {
      flags.push(`[${p}]`);
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
