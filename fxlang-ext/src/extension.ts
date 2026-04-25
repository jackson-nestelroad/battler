import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

interface MemberData {
    description: string;
    type: string;
}

interface ParameterData {
    name: string;
    type: string;
    description: string;
    optional: boolean;
}

interface FlagData {
    name: string;
    description: string;
}

interface FunctionData {
    description: string;
    parameters: ParameterData[];
    flags: FlagData[];
    type: string;
}

interface VariableData {
    type: string;
    optional: boolean;
}

interface Metadata {
    variables: Record<string, VariableData>;
    variable_members: Record<string, MemberData>;
    type_members: Record<string, Record<string, MemberData>>;
    functions: Record<string, FunctionData>;
    events: Record<string, { description: string, variables: Record<string, VariableData> }>;
    common_flags: string[];
}

type SymbolTable = Record<string, string>;

export function activate(context: vscode.ExtensionContext) {
    const metadataPath = path.join(context.extensionPath, 'metadata.json');
    let metadata: Metadata = {
        variables: {},
        variable_members: {},
        type_members: {},
        functions: {},
        events: {},
        common_flags: []
    };

    function loadMetadata() {
        if (fs.existsSync(metadataPath)) {
            try {
                metadata = JSON.parse(fs.readFileSync(metadataPath, 'utf8'));
            } catch (e) {
                console.error('Failed to load fxlang metadata', e);
            }
        }
    }

    loadMetadata();

    const watcher = vscode.workspace.createFileSystemWatcher(metadataPath);
    watcher.onDidChange(() => loadMetadata());
    context.subscriptions.push(watcher);

    /**
     * Checks if the current position is likely within an fxlang code block.
     */
    function isFxLangContext(document: vscode.TextDocument, position: vscode.Position): boolean {
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
    function resolveEventName(rawName: string): string | undefined {
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
    function getEnclosingEvent(document: vscode.TextDocument, position: vscode.Position): string | undefined {
        for (let i = position.line; i >= 0; i--) {
            const line = document.lineAt(i).text.trim();
            const match = line.match(/^"([a-zA-Z0-9_]+)"\s*:\s*[\[{]/);
            if (match) {
                return resolveEventName(match[1]);
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
    function inferType(expression: string, symbols: SymbolTable, eventName?: string): string | undefined {
        expression = expression.trim();
        
        // 1. Literals
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
            return resolveType(chain, symbols, eventName);
        }
        
        return undefined;
    }

    /**
     * Parses the JSON structure up to the cursor to determine if the immediate enclosing block is an array or object.
     * This is used to completely disjoint FxLang program suggestions (which only occur in arrays) 
     * from event callback key suggestions (which only occur in objects).
     */
    function getEnclosingBlockType(document: vscode.TextDocument, position: vscode.Position): 'array' | 'object' | 'none' {
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
    function parseContext(document: vscode.TextDocument, position: vscode.Position): SymbolTable {
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
                
                // If we are on the current line, only parse up to the cursor
                if (i === position.line) {
                    const cursorInLine = position.character - (document.lineAt(i).text.length - line.length);
                    expression = expression.substring(0, cursorInLine).trim();
                }

                const eventName = getEnclosingEvent(document, position);
                const type = inferType(expression, symbols, eventName);
                if (type && type !== 'Undefined') {
                    // Variables cannot change type once set
                    if (!symbols[varName]) {
                        symbols[varName] = type;
                    }
                }
            }
        }
        
        return symbols;
    }

    /**
     * Retrieves all members for a type, taking inheritance into account.
     */
    function getTypeMembers(type: string): Record<string, MemberData> {
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
    function resolveType(chain: string[], symbols: SymbolTable = {}, eventName?: string): string | undefined {
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
            
            const typeMembers: Record<string, MemberData> = getTypeMembers(currentType);
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

    function getChainAtPosition(document: vscode.TextDocument, position: vscode.Position): string[] {
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

    const languages = ['fxlang', 'json', 'jsonc'];

    // Completion Provider
    const completionProvider = vscode.languages.registerCompletionItemProvider(
        languages,
        {
            provideCompletionItems(document: vscode.TextDocument, position: vscode.Position) {
                if (!isFxLangContext(document, position)) return undefined;
                if (getEnclosingBlockType(document, position) !== 'array') return undefined;

                const symbols = parseContext(document, position);
                const linePrefix = document.lineAt(position).text.substring(0, position.character);
                
                // Member completion (after .)
                const memberMatch = linePrefix.match(/\.([\w]*)$/);
                if (memberMatch) {
                    const chain = getChainAtPosition(document, position);
                    const eventName = getEnclosingEvent(document, position);
                    const type = resolveType(chain, symbols, eventName);
                    
                    let items: vscode.CompletionItem[] = [];
                    if (type) {
                        const typeMembers = getTypeMembers(type);
                        items = Object.entries(typeMembers).map(([name, data]) => {
                            const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Field);
                            item.documentation = new vscode.MarkdownString(data.description);
                            item.detail = `(Member of ${type} -> ${data.type})`;
                            return item;
                        });
                    }

                    // Add global variable members (shared across types, e.g., is_undefined)
                    const globalItems = Object.entries(metadata.variable_members).map(([name, data]) => {
                        const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Field);
                        item.documentation = new vscode.MarkdownString(data.description);
                        item.detail = `(Global -> ${data.type})`;
                        return item;
                    });
                    items.push(...globalItems);

                    // Add functions that take this type as an optional first parameter (pseudo-methods)
                    // They expand into a global function call: $mon.heal -> heal: $mon ${1:amount}
                    if (type) {
                        const varName = chain.join('.'); // e.g., $target
                        for (const [name, data] of Object.entries(metadata.functions)) {
                            const firstParam = data.parameters[0];
                            if (firstParam && firstParam.optional && (firstParam.type === type || firstParam.type === 'Any')) {
                                const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Method);
                                item.sortText = ' ' + name;
                                item.documentation = new vscode.MarkdownString(data.description);
                                
                                // To replace the entire "$target.func", we must set the range to cover it.
                                const wordRange = document.getWordRangeAtPosition(position, /[\$a-zA-Z0-9_.]+/);
                                if (wordRange) {
                                    item.range = wordRange;
                                }

                                // CRITICAL: Because we are replacing the entire "$target.func" range, 
                                // VS Code will filter against "$target.func". If our label is just "func", 
                                // it will drop the item. We MUST set filterText to match what the user is typing.
                                item.filterText = `${varName}.${name}`;

                                // Escape $ in varName so VS Code doesn't treat it as a snippet variable
                                const escapedVarName = varName.replace(/\$/g, '\\$');
                                const remainingParams = data.parameters.slice(1);
                                const snippetParams = remainingParams.map((p, i) => `\${${i + 1}:${p.name}}`).join(' ');
                                item.insertText = new vscode.SnippetString(`${name}: ${escapedVarName}${snippetParams ? ' ' + snippetParams : ''}`);
                                
                                const paramsText = data.parameters.map(p => p.optional ? `[${p.name}: ${p.type}]` : `${p.name}: ${p.type}`).join(', ');
                                item.detail = `(Function) ${name}(${paramsText}) -> ${data.type}`;
                                items.push(item);
                            }
                        }
                    }

                    return items;
                }

                // Variable completion (after $)
                const varMatch = linePrefix.match(/\$([\w]*)$/);
                if (varMatch) {
                    const items: vscode.CompletionItem[] = [];
                    
                    const eventName = getEnclosingEvent(document, position);

                    for (const [name, vData] of Object.entries(metadata.variables)) {
                        const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
                        item.sortText = ' ' + name;
                        item.detail = `(Global: ${vData.type})${vData.optional ? ' (optional)' : ''}`;
                        items.push(item);
                    }
                    
                    // Add event-specific built-in variables
                    if (eventName && metadata.events && metadata.events[eventName] && metadata.events[eventName].variables) {
                        for (const [name, vData] of Object.entries(metadata.events[eventName].variables)) {
                            const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
                            item.sortText = ' ' + name;
                            item.detail = `(Context: ${vData.type})${vData.optional ? ' (optional)' : ''}`;
                            items.push(item);
                        }
                    }
                    
                    // Add local variables from static analysis
                    for (const [name, type] of Object.entries(symbols)) {
                        const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
                        item.sortText = ' ' + name;
                        item.detail = `(Local: ${type})`;
                        items.push(item);
                    }
                    
                    return items;
                }

                const wordRange = document.getWordRangeAtPosition(position, /[a-zA-Z0-9_]+/) || new vscode.Range(position, position);

                const items: vscode.CompletionItem[] = [];
                for (const [name, data] of Object.entries(metadata.functions)) {
                    const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Function);
                    item.sortText = ' ' + name;
                    item.range = wordRange;
                    const params = data.parameters.map(p => p.optional ? `[${p.name}: ${p.type}]` : `${p.name}: ${p.type}`).join(', ');
                    item.detail = `(Function) ${name}(${params}) -> ${data.type}`;
                    item.documentation = new vscode.MarkdownString(data.description);
                    
                    // Smart Snippets: Create placeholders for all parameters (including optional)
                    const snippetParams = data.parameters.map((p, i) => `\${${i + 1}:${p.name}}`).join(' ');
                    item.insertText = new vscode.SnippetString(`${name}${snippetParams ? ': ' + snippetParams : ''}`);
                    
                    items.push(item);
                }
                
                const keywords = ['if', 'else', 'foreach', 'in', 'return', 'break', 'continue', 'and', 'or', 'has', 'hasany', 'func_call', 'expr', 'str'];
                for (const kw of keywords) {
                    const item = new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword);
                    item.sortText = ' ' + kw;
                    item.range = wordRange;
                    
                    if (kw === 'func_call') {
                        item.insertText = new vscode.SnippetString('func_call($0)');
                        item.command = { command: 'editor.action.triggerSuggest', title: 'Suggest' };
                    }
                    
                    items.push(item);
                }

                const constants = ['true', 'false', 'undefined', 'stop', ...(metadata.common_flags || [])];
                for (const c of constants) {
                    const item = new vscode.CompletionItem(c, vscode.CompletionItemKind.Constant);
                    item.sortText = ' ' + c;
                    item.detail = "(Flag / Constant)";
                    item.range = wordRange;
                    items.push(item);
                }

                return items;
            }
        },
        '.', '$', '"', '('
    );

    // Hover Provider
    const hoverProvider = vscode.languages.registerHoverProvider(
        languages,
        {
            provideHover(document: vscode.TextDocument, position: vscode.Position) {
                try {
                    if (!isFxLangContext(document, position)) return undefined;

                    const symbols = parseContext(document, position);
                    const wordRange = document.getWordRangeAtPosition(position, /[\$a-zA-Z0-9_]+/);
                    if (!wordRange) return null;

                    const word = document.getText(wordRange);
                    const eventName = getEnclosingEvent(document, position);
                    
                    if (word.startsWith('$')) {
                        const varName = word.substring(1);
                        let type = symbols[varName];
                        let origin = 'Local';
                        let optional = false;
                        
                        if (!type) {
                            if (eventName && metadata.events && metadata.events[eventName] && metadata.events[eventName].variables[varName]) {
                                type = metadata.events[eventName].variables[varName].type;
                                optional = metadata.events[eventName].variables[varName].optional;
                                origin = `Event Context: ${eventName}`;
                            } else if (metadata.variables[varName]) {
                                type = metadata.variables[varName].type;
                                optional = metadata.variables[varName].optional;
                                origin = 'Built-in / Global';
                            }
                        }
                        
                        if (type) {
                            return new vscode.Hover(new vscode.MarkdownString(`**Variable \`${word}\`** (${origin})\n\nType: \`${type}\`${optional ? ' (optional)' : ''}`));
                        }
                    } else {
                        const fullRange = document.getWordRangeAtPosition(position, /[\$a-zA-Z0-9_.]+/);
                        if (fullRange) {
                            const fullText = document.getText(fullRange);
                            const chain = fullText.split('.');
                            const wordIndex = chain.indexOf(word);
                            if (wordIndex > 0) {
                                const lastMember = chain[wordIndex];
                                const parentChain = chain.slice(0, wordIndex);
                                const parentType = resolveType(parentChain, symbols, eventName);
                                
                                if (parentType) {
                                    const typeMembers = getTypeMembers(parentType);
                                    const memberData = typeMembers[lastMember];
                                    if (memberData) {
                                        return new vscode.Hover(new vscode.MarkdownString(`**Member \`${lastMember}\`** of \`${parentType}\`\n\nType: \`${memberData.type}\`\n\n${memberData.description}`));
                                    }
                                }
                                
                                if (metadata.variable_members[lastMember]) {
                                    const memberData = metadata.variable_members[lastMember];
                                    return new vscode.Hover(new vscode.MarkdownString(`**Member \`${lastMember}\`** (Global)\n\nType: \`${memberData.type}\`\n\n${memberData.description}`));
                                }
                            }
                        }
                    }

                const wordRange2 = document.getWordRangeAtPosition(position);
                if (!wordRange2) return null;
                const word2 = document.getText(wordRange2);
                if (metadata.functions[word2]) {
                    const data = metadata.functions[word2];
                    const params = data.parameters.map(p => p.optional ? `[${p.name}: ${p.type}]` : `${p.name}: ${p.type}`).join(', ');
                    const paramDetails = data.parameters.map(p => `* \`${p.name}\`: \`${p.type}\`${p.optional ? ' (optional)' : ''} - ${p.description}`).join('\n');
                    const flagDetails = (data.flags || []).map(f => `* \`${f.name}\` - ${f.description}`).join('\n');
                    
                    const hoverText = new vscode.MarkdownString();
                    hoverText.appendMarkdown(`**Function \`${word2}\`**\n\n`);
                    hoverText.appendCodeblock(`${word2}(${params}) -> ${data.type}`, 'fxlang');
                    hoverText.appendMarkdown(`\n\n${data.description}`);
                    if (paramDetails) {
                        hoverText.appendMarkdown(`\n\n**Parameters:**\n${paramDetails}`);
                    }
                    if (flagDetails) {
                        hoverText.appendMarkdown(`\n\n**Flags:**\n${flagDetails}`);
                    }
                    
                    return new vscode.Hover(hoverText);
                }
                
                const eventKey = resolveEventName(word2);
                if (eventKey && metadata.events[eventKey]) {
                    const eventData = metadata.events[eventKey];
                    const hoverText = new vscode.MarkdownString();
                    hoverText.appendMarkdown(`**Event \`${word2}\`**\n\n`);
                    hoverText.appendMarkdown(`${eventData.description}`);
                    return new vscode.Hover(hoverText);
                }
                } catch (err) {
                    try {
                        const fs = require('fs');
                        const path = require('path');
                        const logPath = path.join(__dirname, '..', 'error_log.txt');
                        fs.appendFileSync(logPath, `${new Date().toISOString()} - ${err instanceof Error ? err.stack : String(err)}\n`);
                    } catch (e) {
                        // Suppress
                    }
                    return undefined;
                }
                return null;
            }
        }
    );
    // Event Callback Autocomplete
    const eventProvider = vscode.languages.registerCompletionItemProvider(
        languages,
        {
            provideCompletionItems(document: vscode.TextDocument, position: vscode.Position) {
                const line = document.lineAt(position.line).text;
                
                let insideCallbacks = false;
                for (let i = position.line; i >= 0; i--) {
                    const l = document.lineAt(i).text.trim();
                    if (l.match(/^"callbacks"\s*:\s*\{/)) {
                        insideCallbacks = true;
                        break;
                    }
                }
                
                if (!insideCallbacks) return undefined;
                if (getEnclosingBlockType(document, position) !== 'object') return undefined;
                
                const textBeforeCursor = line.substring(0, position.character);
                if (textBeforeCursor.match(/^\s*"/)) {
                    const quoteRange = document.getWordRangeAtPosition(position, /"[a-zA-Z0-9_]*"?/) || new vscode.Range(position, position);
                    const items: vscode.CompletionItem[] = [];
                    const modifiers = ['ally', 'any', 'field', 'foe', 'side', 'source'];
                    
                    for (const [baseName, data] of Object.entries(metadata.events || {})) {
                        if (baseName.startsWith('is_') || baseName.startsWith('suppress_')) {
                            // ONLY suggest the base name (no modifiers, no 'on_')
                            const primaryName = baseName;
                            const item = new vscode.CompletionItem(primaryName, vscode.CompletionItemKind.Event);
                            item.sortText = ' ' + primaryName;
                            item.filterText = `"${primaryName}`;
                            item.range = quoteRange;
                            item.documentation = new vscode.MarkdownString(data.description);
                            item.insertText = new vscode.SnippetString(`"${primaryName}": [\n\t$0\n],`);
                            items.push(item);
                        } else {
                            // Standard event: ONLY suggest forms starting with 'on_'
                            const primaryName = 'on_' + baseName;
                            const item = new vscode.CompletionItem(primaryName, vscode.CompletionItemKind.Event);
                            item.sortText = ' ' + primaryName;
                            item.filterText = `"${primaryName}`;
                            item.range = quoteRange;
                            item.documentation = new vscode.MarkdownString(data.description);
                            item.insertText = new vscode.SnippetString(`"${primaryName}": [\n\t$0\n],`);
                            items.push(item);
                            
                            // Modifiers (must include 'on_')
                            for (const mod of modifiers) {
                                const modName = `on_${mod}_${baseName}`;
                                const modItem = new vscode.CompletionItem(modName, vscode.CompletionItemKind.Event);
                                modItem.sortText = '000_' + modName;
                                modItem.filterText = `"${modName}`;
                                modItem.range = quoteRange;
                                modItem.documentation = new vscode.MarkdownString(`*(Modifier: ${mod})*\n\n` + data.description);
                                modItem.insertText = new vscode.SnippetString(`"${modName}": [\n\t$0\n],`);
                                items.push(modItem);
                            }
                        }
                    }
                    return items;
                }
                return undefined;
            }
        },
        '"'
    );

    context.subscriptions.push(completionProvider, hoverProvider, eventProvider);
}

export function deactivate() {}
