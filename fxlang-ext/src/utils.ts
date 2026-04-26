import * as vscode from 'vscode';
import { Metadata, SymbolTable, MemberData } from './types';

export interface FxLangBlock {
    startLine: number;
    endLine: number;
}

export interface FxLangLineMapping {
    documentLine: number;
    charStart: number;
    charEnd: number;
    lineIndex: number;
}

export interface FxLangParseResult {
    blocks: FxLangBlock[];
    mappings: FxLangLineMapping[];
}

export function parseFxLangDocument(document: vscode.TextDocument, metadata: Metadata): FxLangParseResult {
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
            const match = trimmed.match(/^"([a-zA-Z0-9_]+)"\s*:\s*\[/);
            if (match) {
                const rawName = match[1];
                if (resolveEventName(rawName, metadata) || rawName === 'program') {
                    insideFxLangArray = true;
                    fxLangBracketDepth = 1;
                    currentLineIndex = 1;
                    currentBlockStart = i;

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
    return { blocks, mappings };
}

/**
 * Checks if the current position is likely within an fxlang code block.
 */
export function isFxLangContext(document: vscode.TextDocument, position: vscode.Position, metadata: Metadata): boolean {
    if (document.languageId === 'fxlang') return true;
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
    let bestMatch: string | undefined = undefined;
    for (const baseName of Object.keys(metadata.events || {})) {
        if (rawName === baseName || rawName.endsWith('_' + baseName)) {
            if (!bestMatch || baseName.length > bestMatch.length) {
                bestMatch = baseName;
            }
        }
    }
    return bestMatch;
}

/**
 * Walks backwards from the current position to find the enclosing event key inside a callbacks block.
 * Extracts the base event name ignoring prefixes like `on_` and modifiers like `ally_`.
 */
export function getEnclosingEvent(document: vscode.TextDocument, position: vscode.Position, metadata: Metadata): string | undefined {
    for (let i = position.line; i >= 0; i--) {
        const line = document.lineAt(i).text.trim();
        const match = line.match(/^"([a-zA-Z0-9_]+)"\s*:\s*[\[{]/);
        if (match) {
            const rawName = match[1];
            if (rawName !== 'program') {
                return resolveEventName(rawName, metadata);
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
    if (expression.match(/^['"]/)) return 'String';
    if (expression.startsWith('[') && expression.endsWith(']')) {
        const innerText = expression.substring(1, expression.length - 1).trim();
        if (!innerText) return 'List';
        
        const elements = innerText.split(',').map(e => e.trim());
        let commonType: string | undefined = undefined;
        let uniform = true;
        
        for (const element of elements) {
            const elType = inferType(element, symbols, metadata, eventName);
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
                if (argsString) {
                    const firstArg = argsString.split(',')[0].trim();
                    const listType = inferType(firstArg, symbols, metadata, eventName);
                    if (listType && listType.startsWith('List<') && listType.endsWith('>')) {
                        return listType.substring(5, listType.length - 1);
                    }
                }
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

/**
 * Parses the JSON structure up to the cursor to determine if the immediate enclosing block is an array or object.
 * This is used to completely disjoint FxLang program suggestions (which only occur in arrays) 
 * from event callback key suggestions (which only occur in objects).
 */
export function getEnclosingBlockType(document: vscode.TextDocument, position: vscode.Position): 'array' | 'object' | 'none' {
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
    return stack.length > 0 ? stack[stack.length - 1] : 'none';
}

/**
 * Parses the current code block to build a local symbol table (variable type tracking).
 */
export function parseContext(document: vscode.TextDocument, position: vscode.Position, metadata: Metadata, parseUpToCursor = false): SymbolTable {
    const symbols: SymbolTable = {};
    
    // Find the start of the current program/callback block
    let blockStartLine = -1;
    for (let i = position.line; i >= 0; i--) {
        const line = document.lineAt(i).text;
        if (line.includes('"program"') || line.includes('"callbacks"')) {
            blockStartLine = i;
            break;
        }
    }
    
    if (blockStartLine === -1) return symbols;

    // Extract lines from blockStart to current position
    for (let i = blockStartLine; i <= position.line; i++) {
        let line = document.lineAt(i).text.trim();
        
        // Clean up JSON noise (leading quotes, trailing commas/quotes)
        line = line.replace(/^"/, '').replace(/",?$/, '');
        
        // Look for assignments: $var = expression
        // Supporting both "$var = ..." and "set $var = ..."
        const assignMatch = line.match(/(?:set\s+)?(\$[a-zA-Z0-9_]+)\s*=\s*(.*)/);
        if (assignMatch) {
            const varName = assignMatch[1].substring(1);
            let expression = assignMatch[2].trim();
            
            // If we are on the current line and completing, only parse up to the cursor
            if (parseUpToCursor && i === position.line) {
                const cursorInLine = position.character - (document.lineAt(i).text.length - line.length);
                expression = expression.substring(0, cursorInLine).trim();
            }

            const eventName = getEnclosingEvent(document, position, metadata);
            const type = inferType(expression, symbols, metadata, eventName);
            if (type) {
                // Variables cannot change type once set
                if (!symbols[varName]) {
                    symbols[varName] = type;
                }
            }
        }
        
        const foreachMatch = line.match(/foreach\s+(\$[a-zA-Z0-9_]+)\s+in\s*(.*)/);
        if (foreachMatch) {
            const varName = foreachMatch[1].substring(1);
            let expression = foreachMatch[2].replace(/\s*:\s*$/, '').trim();

            const eventName = getEnclosingEvent(document, position, metadata);
            const listType = inferType(expression, symbols, metadata, eventName);
            
            let innerType = 'unknown';
            if (listType) {
                const genericMatch = listType.match(/^List<(.+)>$/);
                if (genericMatch) {
                    innerType = genericMatch[1];
                }
            }
            
            if (!symbols[varName]) {
                symbols[varName] = innerType;
            }
        }
    }
    
    return symbols;
}

/**
 * Retrieves all members for a type, taking inheritance into account.
 */
export function getTypeMembers(type: string, metadata: Metadata): Record<string, MemberData> {
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
    const specificMembers = metadata.type_members[type];
    if (specificMembers) {
        Object.assign(members, specificMembers);
    }
    return members;
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
            if (eventName && metadata.events && metadata.events[eventName] && metadata.events[eventName].variables[varName]) {
                currentType = metadata.events[eventName].variables[varName].type;
            } else if (metadata.variables[varName]) {
                currentType = metadata.variables[varName].type;
            }
        }
    } else {
        const member = metadata.variable_members[first];
        if (member) currentType = member.type;
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
        
        currentType = member.type;
    }

    return currentType;
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
