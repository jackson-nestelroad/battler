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

interface Metadata {
    variables: Record<string, string>;
    variable_members: Record<string, MemberData>;
    type_members: Record<string, Record<string, MemberData>>;
    functions: Record<string, FunctionData>;
    events: Record<string, { description: string }>;
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
        
        const limit = Math.max(0, position.line - 50); // Increased limit for better context
        for (let i = position.line; i >= limit; i--) {
            const line = document.lineAt(i).text;
            if (line.includes('"program"') || line.includes('"callbacks"')) return true;
        }
        return false;
    }

    /**
     * Infers the type of an expression based on literals, function calls, and variable chains.
     */
    function inferType(expression: string, symbols: SymbolTable): string | undefined {
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
            return resolveType(chain, symbols);
        }
        
        return undefined;
    }

    /**
     * Parses the current code block to build a local symbol table (variable type tracking).
     */
    function parseContext(document: vscode.TextDocument, position: vscode.Position): SymbolTable {
        const symbols: SymbolTable = {};
        
        // Find the start of the current program/callback block
        let blockStartLine = -1;
        for (let i = position.line; i >= 0 && i > position.line - 100; i--) {
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

                const type = inferType(expression, symbols);
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
     * Resolves the type of a variable or member access chain.
     */
    function resolveType(chain: string[], symbols: SymbolTable = {}): string | undefined {
        if (chain.length === 0) return undefined;

        let currentType: string | undefined;

        const first = chain[0];
        if (first.startsWith('$')) {
            const varName = first.substring(1);
            // Check local symbol table (static analysis) first, then built-in variables
            currentType = symbols[varName] || metadata.variables[varName];
        } else {
            const member = metadata.variable_members[first];
            if (member) currentType = member.type;
        }

        if (!currentType) return undefined;

        // Resolve remaining parts
        for (let i = 1; i < chain.length; i++) {
            const memberName = chain[i];
            if (!currentType) return undefined;
            
            const typeMembers: Record<string, MemberData> | undefined = metadata.type_members[currentType];
            if (!typeMembers) return undefined;
            
            const member: MemberData | undefined = typeMembers[memberName];
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

                const symbols = parseContext(document, position);
                const linePrefix = document.lineAt(position).text.substring(0, position.character);
                
                // Member completion (after .)
                const memberMatch = linePrefix.match(/\.([\w]*)$/);
                if (memberMatch) {
                    const chain = getChainAtPosition(document, position);
                    const type = resolveType(chain, symbols);
                    
                    let items: vscode.CompletionItem[] = [];
                    if (type && metadata.type_members[type]) {
                        items = Object.entries(metadata.type_members[type]).map(([name, data]) => {
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
                    
                    // Add built-in variables
                    for (const [name, type] of Object.entries(metadata.variables)) {
                        const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
                        item.detail = `(Built-in: ${type})`;
                        items.push(item);
                    }
                    
                    // Add local variables from static analysis
                    for (const [name, type] of Object.entries(symbols)) {
                        const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
                        item.detail = `(Local: ${type})`;
                        items.push(item);
                    }
                    
                    return items;
                }

                const items: vscode.CompletionItem[] = [];
                for (const [name, data] of Object.entries(metadata.functions)) {
                    const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Function);
                    const params = data.parameters.map(p => p.optional ? `[${p.name}: ${p.type}]` : `${p.name}: ${p.type}`).join(', ');
                    item.detail = `(Function) ${name}(${params}) -> ${data.type}`;
                    item.documentation = new vscode.MarkdownString(data.description);
                    
                    // Smart Snippets: Create placeholders for all parameters (including optional)
                    const snippetParams = data.parameters.map((p, i) => `\${${i + 1}:${p.name}}`).join(' ');
                    item.insertText = new vscode.SnippetString(`${name}${snippetParams ? ': ' + snippetParams : ''}`);
                    
                    items.push(item);
                }
                
                const keywords = ['if', 'else', 'elseif', 'foreach', 'in', 'return', 'break', 'continue', 'set'];
                for (const kw of keywords) {
                    items.push(new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword));
                }

                const constants = ['true', 'false', 'undefined', 'stop', ...(metadata.common_flags || [])];
                for (const c of constants) {
                    const item = new vscode.CompletionItem(c, vscode.CompletionItemKind.Constant);
                    item.detail = "(Flag / Constant)";
                    items.push(item);
                }

                return items;
            }
        },
        '.', '$'
    );

    // Hover Provider
    const hoverProvider = vscode.languages.registerHoverProvider(
        languages,
        {
            provideHover(document: vscode.TextDocument, position: vscode.Position) {
                if (!isFxLangContext(document, position)) return undefined;

                const symbols = parseContext(document, position);
                const range = document.getWordRangeAtPosition(position, /[\$a-zA-Z0-9_.]+/);
                if (!range) return null;

                const text = document.getText(range);
                
                if (text.includes('$') || text.includes('.')) {
                    const chain = text.split('.');
                    const resolvedType = resolveType(chain, symbols);
                    
                    if (chain.length === 1 && chain[0].startsWith('$')) {
                        const varName = chain[0].substring(1);
                        const type = symbols[varName] || metadata.variables[varName];
                        if (type) {
                            const origin = symbols[varName] ? 'Local' : 'Built-in';
                            return new vscode.Hover(new vscode.MarkdownString(`**Variable \`${chain[0]}\`** (${origin})\n\nType: \`${type}\``));
                        }
                    }

                    const lastMember = chain[chain.length - 1];
                    const parentChain = chain.slice(0, -1);
                    const parentType = resolveType(parentChain, symbols);
                    
                    if (parentType && metadata.type_members[parentType]) {
                        const memberData = metadata.type_members[parentType][lastMember];
                        if (memberData) {
                            return new vscode.Hover(new vscode.MarkdownString(`**Member \`${lastMember}\`** of \`${parentType}\`\n\nType: \`${memberData.type}\`\n\n${memberData.description}`));
                        }
                    }
                }

                const wordRange = document.getWordRangeAtPosition(position);
                if (!wordRange) return null;
                const word = document.getText(wordRange);
                if (metadata.functions[word]) {
                    const data = metadata.functions[word];
                    const params = data.parameters.map(p => p.optional ? `[${p.name}: ${p.type}]` : `${p.name}: ${p.type}`).join(', ');
                    const paramDetails = data.parameters.map(p => `* \`${p.name}\`: \`${p.type}\`${p.optional ? ' (optional)' : ''} - ${p.description}`).join('\n');
                    const flagDetails = (data.flags || []).map(f => `* \`${f.name}\` - ${f.description}`).join('\n');
                    
                    const hoverText = new vscode.MarkdownString();
                    hoverText.appendMarkdown(`**Function \`${word}\`**\n\n`);
                    hoverText.appendCodeblock(`${word}(${params}) -> ${data.type}`, 'fxlang');
                    hoverText.appendMarkdown(`\n\n${data.description}`);
                    if (paramDetails) {
                        hoverText.appendMarkdown(`\n\n**Parameters:**\n${paramDetails}`);
                    }
                    if (flagDetails) {
                        hoverText.appendMarkdown(`\n\n**Flags:**\n${flagDetails}`);
                    }
                    
                    return new vscode.Hover(hoverText);
                }

                return null;
            }
        }
    );

    context.subscriptions.push(completionProvider, hoverProvider);
}

export function deactivate() {}
