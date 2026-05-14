import * as vscode from 'vscode';
import { Metadata, SymbolTable, MemberData, FxLangParseResult, FxLangBlock, FxLangLineMapping } from './types';

const relevanceCache = new Map<string, { version: number, relevant: boolean }>();
let eventNamesSet = new Set<string>();
let typeMembersCache: Record<string, Record<string, MemberData>> = {};
let commonFlagsSet = new Set<string>();
const typeSplitCache = new Map<string, Set<string>>();

function getSplitTypes(typeStr: string): Set<string> {
    let cached = typeSplitCache.get(typeStr);
    if (cached) return cached;
    const split = new Set(typeStr.split(' | '));
    typeSplitCache.set(typeStr, split);
    return split;
}

export function preprocessMetadata(metadata: Metadata) {
    updateEventNamesSet(metadata);
    commonFlagsSet = new Set(metadata.common_flags || []);
    
    // Pre-merge type members
    typeMembersCache = {};
    for (const type of Object.keys(metadata.type_members)) {
        typeMembersCache[type] = internalGetTypeMembers(type, metadata);
    }
    // Special case for ActiveMove inheritance
    typeMembersCache['ActiveMove'] = internalGetTypeMembers('ActiveMove', metadata);
}

export function updateEventNamesSet(metadata: Metadata) {
    eventNamesSet = new Set(Object.keys(metadata.events || {}));
}

export function isRelevantDocument(document: vscode.TextDocument): boolean {
    if (document.languageId === 'fxlang') return true;
    if (document.languageId !== 'json' && document.languageId !== 'jsonc') return false;
    
    const uri = document.uri.toString();
    const cached = relevanceCache.get(uri);
    if (cached && cached.version === document.version) return cached.relevant;
    
    const text = document.getText();
    const relevant = text.includes('"callbacks"') || text.includes('"program"');
    relevanceCache.set(uri, { version: document.version, relevant });
    return relevant;
}

let parseCache: { [uri: string]: { version: number, result: FxLangParseResult } } = {};

export function parseFxLangDocument(document: vscode.TextDocument, metadata: Metadata): FxLangParseResult {
    const uri = document.uri.toString();
    const version = document.version;
    
    if (parseCache[uri] && parseCache[uri].version === version) {
        return parseCache[uri].result;
    }

    if (!isRelevantDocument(document)) {
        const result = { blocks: [], mappings: [] };
        parseCache[uri] = { version, result };
        return result;
    }

    const blocks: FxLangBlock[] = [];
    const mappings: FxLangLineMapping[] = [];
    
    let insideFxLangArray = false;
    let fxLangBracketDepth = 0;
    let currentLineIndex = 1;
    let currentBlockStart = -1;

    for (let i = 0; i < document.lineCount; i++) {
        const line = document.lineAt(i).text;
        const trimmed = line.trim();

        if (!insideFxLangArray) {
            // Fast path for non-array-start lines
            if (!trimmed.startsWith('"') || !trimmed.includes('[')) continue;

            const match = trimmed.match(/^"([a-zA-Z0-9_]+)"\s*:\s*\[/);
            if (match) {
                const rawName = match[1];
                const resolved = resolveEventName(rawName, metadata);
                if (resolved || rawName === 'program') {
                    insideFxLangArray = true;
                    fxLangBracketDepth = 1;
                    currentLineIndex = 1;
                    currentBlockStart = i;

                    if (rawName === 'program') {
                        for (let j = i - 1; j >= 0; j--) {
                            const prevLine = document.lineAt(j).text.trim();
                            const prevMatch = prevLine.match(/^"([a-zA-Z0-9_]+)"\s*:\s*[\[\{]/);
                            if (prevMatch) {
                                const prevRaw = prevMatch[1];
                                if (resolveEventName(prevRaw, metadata) && prevRaw !== 'program' && prevRaw !== 'metadata') {
                                    currentBlockStart = j;
                                    break;
                                }
                            }
                        }
                    }

                    if (trimmed.endsWith(']') || trimmed.endsWith('],')) {
                        const stringMatches = trimmed.match(/"([^"]*)"/g);
                        if (stringMatches && stringMatches.length > 1) {
                            let lastIdx = 0;
                            for (let s = 1; s < stringMatches.length; s++) {
                                const str = stringMatches[s].replace(/^"/, '').replace(/"$/, '');
                                const strIdx = line.indexOf('"' + str + '"', lastIdx);
                                if (strIdx !== -1) {
                                    lastIdx = strIdx + str.length + 2;
                                    mappings.push({
                                        documentLine: i,
                                        charStart: strIdx,
                                        charEnd: strIdx + str.length + 2,
                                        lineIndex: s
                                    });
                                }
                            }
                        }
                        blocks.push({ startLine: currentBlockStart, endLine: i });
                        insideFxLangArray = false;
                    }
                }
            }
        } else {
            for (const char of trimmed) {
                if (char === '[') fxLangBracketDepth++;
                if (char === ']') fxLangBracketDepth--;
            }

            if (fxLangBracketDepth <= 0) {
                blocks.push({ startLine: currentBlockStart, endLine: i });
                insideFxLangArray = false;
                continue;
            }

            const stringMatches = trimmed.match(/"([^"]*)"/g);
            if (stringMatches) {
                let lastIdx = 0;
                for (let s = 0; s < stringMatches.length; s++) {
                    const str = stringMatches[s].replace(/^"/, '').replace(/"$/, '');
                    const strIdx = line.indexOf('"' + str + '"', lastIdx);
                    if (strIdx !== -1) {
                        lastIdx = strIdx + str.length + 2;
                        mappings.push({
                            documentLine: i,
                            charStart: strIdx,
                            charEnd: strIdx + str.length + 2,
                            lineIndex: currentLineIndex
                        });
                        currentLineIndex++;
                    }
                }
            }
        }
    }
    if (insideFxLangArray) {
        blocks.push({ startLine: currentBlockStart, endLine: document.lineCount - 1 });
    }
    const result = { blocks, mappings };
    parseCache[uri] = { version, result };
    return result;
}

/**
 * Checks if the current position is likely within an fxlang code block.
 */
export function isFxLangContext(document: vscode.TextDocument, position: vscode.Position, metadata: Metadata): boolean {
    if (document.languageId === 'fxlang') return true;
    if (!isRelevantDocument(document)) return false;
    const { blocks } = parseFxLangDocument(document, metadata);
    return blocks.some(b => position.line >= b.startLine && position.line <= b.endLine);
}

/**
 * Checks if the current position is inside an fxlang program string.
 */
export function isInFxLangProgram(document: vscode.TextDocument, position: vscode.Position, metadata: Metadata): boolean {
    if (!isFxLangContext(document, position, metadata)) return false;
    
    const line = document.lineAt(position.line).text;
    const match = line.match(/^(\s*"[a-zA-Z0-9_]+"\s*):/);
    if (match) {
        const keyEndPos = match[1].length;
        if (position.character <= keyEndPos) {
            return false;
        }
    }
    
    return true;
}

/**
 * Finds the longest base event name that is a suffix of the raw JSON key.
 */
export function resolveEventName(rawName: string, metadata: Metadata): string | undefined {
    if (eventNamesSet.size === 0) updateEventNamesSet(metadata);
    if (eventNamesSet.has(rawName)) return rawName;
    
    if (rawName.startsWith('on_')) {
        let base = rawName.substring(3);
        if (eventNamesSet.has(base)) return base;
        
        for (const mod of EVENT_MODIFIERS) {
            if (base.startsWith(mod + '_')) {
                let deeperBase = base.substring(mod.length + 1);
                if (eventNamesSet.has(deeperBase)) return deeperBase;
            }
        }
    }
    
    return undefined;
}
export function getCustomVariables(document: vscode.TextDocument, position: vscode.Position): string[] {
    const params: string[] = [];
    
    for (let i = position.line; i >= 0; i--) {
        const line = document.lineAt(i).text.trim();
        
        if (line.includes('"parameters"')) {
            const match = line.match(/"parameters"\s*:\s*\[(.*?)\]/);
            if (match) {
                const parts = match[1].split(',').map(p => p.trim().replace(/^"/, '').replace(/"$/, ''));
                for (const p of parts) {
                    if (p) params.push(p);
                }
                return params;
            }
            
            const startMatch = line.match(/"parameters"\s*:\s*\[/);
            if (startMatch) {
                for (let j = i + 1; j < document.lineCount; j++) {
                    const forwardLine = document.lineAt(j).text.trim();
                    if (forwardLine.includes(']')) {
                        const beforeBracket = forwardLine.split(']')[0];
                        const parts = beforeBracket.split(',').map(p => p.trim().replace(/^"/, '').replace(/"$/, ''));
                        for (const p of parts) {
                            if (p) params.push(p);
                        }
                        break;
                    }
                    const parts = forwardLine.split(',').map(p => p.trim().replace(/^"/, '').replace(/"$/, ''));
                    for (const p of parts) {
                        if (p) params.push(p);
                    }
                }
                return params;
            }
        }
        
        const eventMatch = line.match(/^"([a-zA-Z0-9_]+)"\s*:\s*[\[{]/);
        if (eventMatch && eventMatch[1] !== 'program' && eventMatch[1] !== 'metadata') {
            break;
        }
    }
    return params;
}


/**
 * Walks backwards from the current position to find the enclosing event key inside a callbacks block.
 * Extracts the base event name ignoring prefixes like `on_` and modifiers like `ally_`.
 */
export function getEnclosingEvent(document: vscode.TextDocument, position: vscode.Position, metadata: Metadata): string | undefined {
    // Limit scan to 500 lines for performance
    const startLine = Math.max(0, position.line - 500);
    for (let i = position.line; i >= startLine; i--) {
        const line = document.lineAt(i).text.trim();
        if (!line.startsWith('"')) continue;
        const match = line.match(/^"([a-zA-Z0-9_]+)"\s*:\s*[\[{]/);
        if (match) {
            const rawName = match[1];
            if (rawName !== 'program' && rawName !== 'metadata') {
                const resolved = resolveEventName(rawName, metadata);
                if (resolved) return resolved;
            }
        }
        if (line.match(/^"callbacks"\s*:\s*\{/)) {
            break;
        }
    }
    return undefined;
}

/**
 * Infers the type of an expression based on literals, function calls, and variable chains.
 */
export function inferType(expression: string, symbols: SymbolTable, metadata: Metadata, eventName?: string): string | undefined {
    expression = expression.trim();
    
    if (expression === 'undefined') return 'Undefined';
    if (expression.match(/^(true|false)$/)) return 'Boolean';
    if (expression.match(/^[+-]\d+(\.\d+)?$/)) return 'Fraction';
    if (expression.match(/^\d+(\.\d+)?$/)) return 'UFraction';
    if (expression.match(/^[+-]?\d+(?:\.\d+)?(?:\s*[\+\-\*\/]\s*\d+(?:\.\d+)?)+$/)) {
        return expression.startsWith('-') ? 'Fraction' : 'UFraction';
    }
    if (expression.match(/^['"]/)) return 'String';
    if (expression.startsWith('[') && expression.endsWith(']')) {
        const innerText = expression.substring(1, expression.length - 1).trim();
        if (!innerText) return 'List';
        
        const elements = innerText.split(',').map(e => e.trim());
        let commonType: string | undefined = undefined;
        let uniform = true;
        
        // Only check first 5 elements for performance
        const checkLimit = Math.min(elements.length, 5);
        for (let i = 0; i < checkLimit; i++) {
            const elType = inferType(elements[i], symbols, metadata, eventName);
            if (!elType || elType === 'unknown') {
                uniform = false;
                break;
            }
            if (commonType === undefined) {
                commonType = elType;
            } else if (commonType !== elType) {
                uniform = false;
                break;
            }
        }
        
        if (uniform && commonType) {
            return `List<${commonType}>`;
        }
        return 'List';
    }
    const funcMatch = expression.match(/^([a-z0-9_]+)\(/);
    if (funcMatch) {
        let funcName = funcMatch[1];
        if (funcName === 'func_call') {
            const innerMatch = expression.match(/^func_call\(\s*([a-z0-9_]+)/);
            if (innerMatch) {
                funcName = innerMatch[1];
            }
        }
        const funcMeta = metadata.functions[funcName];
        if (funcMeta) {
            if (funcMeta.returns_item_from_list) {
                const argsString = extractFunctionArguments(expression);
                if (argsString) {
                    const firstArg = argsString.split(',')[0].trim();
                    const listType = inferType(firstArg, symbols, metadata, eventName);
                    return unwrapListType(listType);
                }
                return 'unknown';
            }
            if (funcMeta.type === 'List' && funcMeta.item_type) {
                return `List<${funcMeta.item_type}>`;
            }
            return funcMeta.type;
        }
    }
    
    // 3. Variable/Member chains: $var.member...
    const chainMatch = expression.match(/^(\$[a-zA-Z0-9_]+(?:\.[a-zA-Z0-9_]+)*)/);
    if (chainMatch) {
        const chain = chainMatch[1].split('.');
        return resolveType(chain, symbols, metadata, eventName);
    }
    
    if (expression.match(/^[a-zA-Z0-9_]+$/)) {
        return 'String';
    }
    
    return undefined;
}

let blockTypeCache: { [uri: string]: { version: number, line: number, character: number, result: 'array' | 'object' | 'none' } } = {};

/**
 * Parses the JSON structure up to the cursor to determine if the immediate enclosing block is an array or object.
 * This is used to completely disjoint FxLang program suggestions (which only occur in arrays) 
 * from event callback key suggestions (which only occur in objects).
 */
export function getEnclosingBlockType(document: vscode.TextDocument, position: vscode.Position): 'array' | 'object' | 'none' {
    const uri = document.uri.toString();
    const version = document.version;
    const { line, character } = position;
    
    if (blockTypeCache[uri] && 
        blockTypeCache[uri].version === version && 
        blockTypeCache[uri].line === line && 
        blockTypeCache[uri].character === character) {
        return blockTypeCache[uri].result;
    }

    const text = document.getText(new vscode.Range(new vscode.Position(0, 0), position));
    const stack: ('array' | 'object')[] = [];
    let inString = false;
    let escape = false;
    
    for (let i = 0; i < text.length; i++) {
        const char = text[i];
        if (escape) {
            escape = false;
            continue;
        }
        if (char === '\\') {
            escape = true;
            continue;
        }
        if (char === '"') {
            inString = !inString;
            continue;
        }
        if (!inString) {
            if (char === '[') stack.push('array');
            else if (char === ']') stack.pop();
            else if (char === '{') stack.push('object');
            else if (char === '}') stack.pop();
        }
    }
    const result = stack.length > 0 ? stack[stack.length - 1] : 'none';
    blockTypeCache[uri] = { version, line, character, result };
    return result;
}

let symbolTableCache: { [uri: string]: { version: number, blockStartLine: number, symbols: SymbolTable } } = {};

/**
 * Parses the current code block to build a local symbol table (variable type tracking).
 */
export function parseContext(document: vscode.TextDocument, position: vscode.Position, metadata: Metadata, parseUpToCursor = false): SymbolTable {
    const uri = document.uri.toString();
    const version = document.version;

    // Find the start of the current program/callback block
    let blockStartLine = -1;
    for (let i = position.line; i >= Math.max(0, position.line - 500); i--) {
        const line = document.lineAt(i).text;
        if (line.match(/"[a-z0-9_]+"\s*:\s*\[/i)) {
            blockStartLine = i;
            break;
        }
    }
    
    if (blockStartLine === -1) return {};

    // Check cache (only if not parsing up to cursor, which is position-specific)
    if (!parseUpToCursor && symbolTableCache[uri] && symbolTableCache[uri].version === version && symbolTableCache[uri].blockStartLine === blockStartLine) {
        return symbolTableCache[uri].symbols;
    }

    const symbols: SymbolTable = {};

    // Extract lines from blockStart to current position
    for (let i = blockStartLine; i <= position.line; i++) {
        let line = document.lineAt(i).text.trim();
        if (!line.includes('$') && !line.includes('foreach')) continue;

        // Clean up JSON noise (leading quotes, trailing commas/quotes)
        line = line.replace(/^"/, '').replace(/",?$/, '');
        
        // Look for assignments: $var = expression
        const assignMatch = line.match(/(?:set\s+)?(\$[a-zA-Z0-9_]+)\s*=\s*(.*)/);
        if (assignMatch) {
            const varName = assignMatch[1].substring(1);
            let expression = assignMatch[2].trim();
            
            if (parseUpToCursor && i === position.line) {
                const cursorInLine = position.character - (document.lineAt(i).text.length - line.length);
                expression = expression.substring(0, cursorInLine).trim();
            }

            const eventName = getEnclosingEvent(document, position, metadata);
            const type = inferType(expression, symbols, metadata, eventName) || 'unknown';
            if (!getVariableData(varName, metadata, eventName) && !symbols[varName]) {
                symbols[varName] = type;
            }
        }
        
        const foreachMatch = line.match(/foreach\s+(\$[a-zA-Z0-9_]+)\s+in\s*(.*)/);
        if (foreachMatch) {
            const varName = foreachMatch[1].substring(1);
            let expression = foreachMatch[2].replace(/\s*:\s*$/, '').trim();

            const eventName = getEnclosingEvent(document, position, metadata);
            const listType = inferType(expression, symbols, metadata, eventName);
            const innerType = unwrapListType(listType);
            
            if (!getVariableData(varName, metadata, eventName) && !symbols[varName]) {
                symbols[varName] = innerType;
            }
        }
    }
    
    if (!parseUpToCursor) {
        symbolTableCache[uri] = { version, blockStartLine, symbols };
    }
    
    return symbols;
}

/**
 * Retrieves all members for a type, taking inheritance into account.
 */
function internalGetTypeMembers(type: string, metadata: Metadata): Record<string, MemberData> {
    const members: Record<string, MemberData> = {};
    if (type.includes(' | ')) {
        type = type.split(' | ')[0];
    }
    if (type === 'ActiveMove') {
        const effectMembers = metadata.type_members['Effect'];
        if (effectMembers) {
            Object.assign(members, effectMembers);
        }
    }
    if (type === 'Effect') {
        const moveMembers = metadata.type_members['ActiveMove'];
        if (moveMembers) {
            for (const [name, data] of Object.entries(moveMembers)) {
                if (!members[name]) {
                    members[name] = data;
                }
            }
        }
    }
    const specificMembers = metadata.type_members[type];
    if (specificMembers) {
        Object.assign(members, specificMembers);
    }
    return members;
}

export function getTypeMembers(type: string, metadata: Metadata): Record<string, MemberData> {
    if (type.includes(' | ')) {
        type = type.split(' | ')[0];
    }
    return typeMembersCache[type] || metadata.type_members[type] || {};
}

/**
 * Resolves the type of a variable or member access chain.
 */
export function resolveType(chain: string[], symbols: SymbolTable, metadata: Metadata, eventName?: string): string | undefined {
    if (chain.length === 0) return undefined;

    let currentType: string | undefined;

    const first = chain[0];
    if (first.startsWith('$')) {
        const varName = first.substring(1);
        // Check local symbol table (static analysis) first
        currentType = symbols[varName];
        if (!currentType) {
            const varData = getVariableData(varName, metadata, eventName);
            if (varData) {
                currentType = getDisplayType(varData.type, varData.item_type);
            }
        }
    } else {
        const member = metadata.variable_members[first];
        if (member) currentType = getDisplayType(member.type, member.item_type);
    }

    // Resolve remaining parts
    for (let i = 1; i < chain.length; i++) {
        const memberName = chain[i];
        if (!currentType) {
            if (metadata.variable_members[memberName]) {
                currentType = metadata.variable_members[memberName].type;
                continue;
            }
            return undefined;
        }
        
        const typeMembers: Record<string, MemberData> = getTypeMembers(currentType, metadata);
        if (Object.keys(typeMembers).length === 0) return undefined;
        
        let member: MemberData | undefined = typeMembers[memberName];
        if (!member && metadata.variable_members[memberName]) {
            member = metadata.variable_members[memberName];
        }
        if (!member) return undefined;
        
        currentType = getDisplayType(member.type, member.item_type);
    }

    return currentType;
}

export function unwrapListType(typeStr: string | undefined): string {
    if (!typeStr) return 'unknown';
    const genericMatch = typeStr.match(/List<([^>]+)>/);
    if (genericMatch) {
        return genericMatch[1];
    }
    return 'unknown';
}

export function extractFunctionArguments(expression: string): string {
    let argsString = '';
    const baseFuncMatch = expression.match(/^([a-z0-9_]+)\((.*)\)$/);
    if (baseFuncMatch) {
        if (baseFuncMatch[1] === 'func_call') {
            const innerStr = baseFuncMatch[2];
            const parts = innerStr.split(':');
            if (parts.length > 1) {
                argsString = parts.slice(1).join(':').trim();
            }
        } else {
            argsString = baseFuncMatch[2].trim();
        }
    }
    return argsString;
}

export function getVariableData(varName: string, metadata: Metadata, eventName?: string) {
    if (eventName && metadata.events && metadata.events[eventName] && metadata.events[eventName].variables[varName]) {
        const vData = metadata.events[eventName].variables[varName];
        return {
            type: vData.type,
            optional: vData.optional,
            item_type: (vData as any).item_type,
            origin: `Event Context: ${eventName}`
        };
    } else if (metadata.variables[varName]) {
        const vData = metadata.variables[varName];
        return {
            type: vData.type,
            optional: vData.optional,
            item_type: (vData as any).item_type,
            origin: 'Built-in / Global'
        };
    }
    return undefined;
}

export function getChainAtPosition(document: vscode.TextDocument, position: vscode.Position): string[] {
    const line = document.lineAt(position).text;
    const prefix = line.substring(0, position.character);
    
    const match = prefix.match(/(\$[a-zA-Z0-9_]+(?:\.[a-zA-Z0-9_]+)*)\.$/);
    if (match) {
        return match[1].split('.');
    }

    const matchInside = prefix.match(/(\$[a-zA-Z0-9_]+(?:\.[a-zA-Z0-9_]+)*)$/);
    if (matchInside) {
        const parts = matchInside[1].split('.');
        return parts.slice(0, -1);
    }

    return [];
}

export const EVENT_MODIFIERS = ['ally', 'any', 'field', 'foe', 'side', 'source'];

export function getDisplayType(typeStr: string, itemType?: string): string {
    if (typeStr.includes('List') && itemType) {
        return typeStr.replace('List', `List<${itemType}>`);
    }
    return typeStr;
}

export function areTypesCompatible(parentType: string, paramType: string): boolean {
    if (paramType === 'Any') return true;
    const parentTypes = getSplitTypes(parentType);
    const paramTypes = getSplitTypes(paramType);
    
    for (const pt of parentTypes) {
        if (paramTypes.has(pt)) return true;
    }
    
    // Check specific relaxed rules
    for (const pt of parentTypes) {
        if (paramTypes.has('Effect') && pt === 'ActiveMove') return true;
        if (paramTypes.has('Object') && (pt === 'BoostTable' || pt === 'StatTable' || pt === 'EffectState')) return true;
        if (paramTypes.has('Fraction') && pt === 'UFraction') return true;
        if (paramTypes.has('UFraction') && pt === 'Fraction') return true;
    }
    
    return false;
}
