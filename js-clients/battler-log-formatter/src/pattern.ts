export interface ParsedPattern {
  title: string;
  tags: string[];
  flags: string[];
}

export function parsePattern(line: string): ParsedPattern {
  const parts = line
    .split("|")
    .map((p) => p.trim())
    .filter(Boolean);
  const title = parts[0] || "";
  const tags: string[] = [];
  const flags: string[] = [];

  for (let i = 1; i < parts.length; i++) {
    const p = parts[i];
    if (p.includes(":")) {
      tags.push(p);
    } else {
      flags.push(p);
    }
  }

  return { title, tags, flags };
}

export function serializePattern(parsed: ParsedPattern): string {
  const sortedTags = [...parsed.tags].sort();
  const sortedFlags = [...parsed.flags].sort();
  return [parsed.title, ...sortedTags, ...sortedFlags].join("|");
}

export function patternToKey(pattern: string): string {
  return pattern
    .replace(/\|/g, "__")
    .replace(/:/g, "_")
    .replace(/\*/g, "any")
    .toLowerCase()
    .replace(/[^a-z0-9_]/g, "");
}

export interface ScraperRule {
  match: Record<string, string | undefined>;
  strip?: string[];
  collapse?: string[];
  inject?: string[];
  except?: Record<string, string | string[] | undefined>;
}

export function matchesRule(tags: string[], title: string, rule: ScraperRule): boolean {
  for (const [matchK, matchV] of Object.entries(rule.match)) {
    if (matchV === undefined) continue;
    if (matchK === "title") {
      if (title !== matchV) return false;
    } else {
      const hasTag = tags.some((t) => {
        if (matchV === "*") {
          return t.startsWith(`${matchK}:`);
        } else if (matchV.endsWith("*")) {
          return t.startsWith(`${matchK}:${matchV.slice(0, -1)}`);
        } else {
          return t === `${matchK}:${matchV}`;
        }
      });
      if (!hasTag) return false;
    }
  }

  if (rule.except) {
    for (const [excK, excV] of Object.entries(rule.except)) {
      if (excV === undefined) continue;
      const excList = Array.isArray(excV) ? excV : [excV];
      const hasExc = tags.some((t) => {
        return excList.some((v: string) => {
          if (v === "*") {
            return t.startsWith(`${excK}:`);
          } else if (v.endsWith("*")) {
            return t.startsWith(`${excK}:${v.slice(0, -1)}`);
          } else {
            return t === `${excK}:${v}`;
          }
        });
      });
      if (hasExc) return false;
    }
  }

  return true;
}
