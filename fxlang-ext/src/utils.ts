import * as vscode from 'vscode';
import { Metadata, SymbolTable, MemberData } from './types';

/**
 * Checks if the current position is likely within an fxlang code block.
 */
export function isFxLangContext(document: vscode.TextDocument, position: vscode.Position): boolean {
    if (document.languageId === 'fxlang') return true;
    
    for (let i = position.line; i >= 0; i--) {
        const line = document.lineAt(i).text;
        if (line.includes('"program"') || line.includes('"callbacks"')) return true;
    }
    return false;
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
            return resolveEventName(match[1], metadata);
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
    if (expression.match(/^-?\d+(\.\d+)?$/)) return 'UFraction';
    if (expression.match(/^['"]/)) return 'String';
    
    
    // 2. Function calls: func(...)
    const funcMatch = expression.match(/^([a-z0-9_]+)\(/);
    if (funcMatch) {
        const funcName = funcMatch[1];
        return metadata.functions[funcName]?.type;
    }
    
    // 3. Variable/Member chains: $var.member...
    const chainMatch = expression.match(/^(\$[a-zA-Z0-9_]+(?:\.[a-zA-Z0-9_]+)*)/);
    if (chainMatch) {
        const chain = chainMatch[1].split('.');
        return resolveType(chain, symbols, metadata, eventName);
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
        
        const foreachMatch = line.match(/foreach\s+(\$[a-zA-Z0-9_]+)\s+in/);
        if (foreachMatch) {
            const varName = foreachMatch[1].substring(1);
            if (!symbols[varName]) {
                symbols[varName] = 'unknown';
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
