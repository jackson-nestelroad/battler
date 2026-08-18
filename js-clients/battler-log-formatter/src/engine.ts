export type TokenType = "text" | "variable";

export interface LogToken {
  type: TokenType;
  value: string;
}

const astCache = new Map<string, LogToken[]>();

export function parseTemplateToTokens(template: string): LogToken[] {
  if (astCache.has(template)) {
    return astCache.get(template)!;
  }

  const tokens: LogToken[] = [];
  const regex = /\{\{([a-zA-Z0-9_]+)\}\}/g;
  let lastIndex = 0;
  
  let match;
  while ((match = regex.exec(template)) !== null) {
    if (match.index > lastIndex) {
      tokens.push({ type: "text", value: template.substring(lastIndex, match.index) });
    }
    tokens.push({ type: "variable", value: match[1] });
    lastIndex = regex.lastIndex;
  }
  
  if (lastIndex < template.length) {
    tokens.push({ type: "text", value: template.substring(lastIndex) });
  }
  
  astCache.set(template, tokens);
  return tokens;
}
