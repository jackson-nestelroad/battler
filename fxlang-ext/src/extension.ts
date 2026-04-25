import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { Metadata, MemberData } from './types';
import { 
    isFxLangContext, 
    isInFxLangProgram,
    resolveEventName, 
    getEnclosingEvent, 
    getEnclosingBlockType, 
    parseContext, 
    getChainAtPosition, 
    resolveType, 
    getTypeMembers 
} from './utils';

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
                updateDecorations();
            } catch (e) {
                console.error('Failed to load fxlang metadata', e);
            }
        }
    }

    loadMetadata();

    const watcher = vscode.workspace.createFileSystemWatcher(metadataPath);
    watcher.onDidChange(() => loadMetadata());
    context.subscriptions.push(watcher);

    const languages = ['fxlang', 'json', 'jsonc'];

    // Completion Provider
    const completionProvider = vscode.languages.registerCompletionItemProvider(
        languages,
        {
            provideCompletionItems(document: vscode.TextDocument, position: vscode.Position) {
                if (!isFxLangContext(document, position)) return undefined;
                if (getEnclosingBlockType(document, position) !== 'array') return undefined;

                const symbols = parseContext(document, position, metadata, true);
                const linePrefix = document.lineAt(position).text.substring(0, position.character);
                
                // Member completion (after .)
                const memberMatch = linePrefix.match(/\.([\w]*)$/);
                if (memberMatch) {
                    const chain = getChainAtPosition(document, position);
                    const eventName = getEnclosingEvent(document, position, metadata);
                    const type = resolveType(chain, symbols, metadata, eventName);
                    
                    let items: vscode.CompletionItem[] = [];
                    if (type) {
                        const typeMembers = getTypeMembers(type, metadata);
                        items = Object.entries(typeMembers).map(([name, data]) => {
                            const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Field);
                            item.documentation = new vscode.MarkdownString(data.description);
                            item.detail = `(Member of ${type} -> ${data.type})`;
                            item.sortText = '0_' + name;
                            return item;
                        });

                        if (type === 'Effect') {
                            const moveMembers = metadata.type_members['ActiveMove'];
                            if (moveMembers) {
                                for (const [name, data] of Object.entries(moveMembers)) {
                                    if (!typeMembers[name]) {
                                        const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Field);
                                        item.documentation = new vscode.MarkdownString(data.description);
                                        item.detail = `(Potential Move Member -> ${data.type})`;
                                        item.sortText = 'z_' + name;
                                        items.push(item);
                                    }
                                }
                            }
                        }
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
                    if (type) {
                        const varName = chain.join('.');
                        for (const [name, data] of Object.entries(metadata.functions)) {
                            const firstParam = data.parameters[0];
                            if (firstParam) {
                                const pType = firstParam.type;
                                const isMatch = pType === type || pType === 'Any' ||
                                    (pType === 'Object' && (type === 'BoostTable' || type === 'StatTable' || type === 'EffectState')) ||
                                    (pType === 'Fraction' && type === 'UFraction') ||
                                    (pType === 'UFraction' && type === 'Fraction');
                                
                                if (isMatch) {
                                const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Method);
                                item.sortText = ' ' + name;
                                item.documentation = new vscode.MarkdownString(data.description);
                                
                                const wordRange = document.getWordRangeAtPosition(position, /[\$a-zA-Z0-9_.]+/);
                                if (wordRange) {
                                    item.range = wordRange;
                                }

                                item.filterText = `${varName}.${name}`;

                                const escapedVarName = varName.replace(/\$/g, '\\$');
                                const remainingParams = data.parameters.slice(1);
                                const snippetParams = remainingParams.map((p, i) => `\${${i + 1}:${p.name}}`).join(' ');
                                item.insertText = new vscode.SnippetString(`${name}: ${escapedVarName}${snippetParams ? ' ' + snippetParams : ''}`);
                                
                                const paramsText = data.parameters.map(p => p.optional ? `[${p.name}: ${p.type}]` : `${p.name}: ${p.type}`).join(', ');
                                const returnTypeStr = (data.type === 'List' && (data as any).item_type) ? `List<${(data as any).item_type}>` : data.type;
                                item.detail = `(Function) ${name}(${paramsText}) -> ${returnTypeStr}`;
                                items.push(item);
                                }
                            }
                        }
                    }

                    return items;
                }

                // Variable completion (after $)
                const varMatch = linePrefix.match(/\$([\w]*)$/);
                if (varMatch) {
                    const items: vscode.CompletionItem[] = [];
                    const eventName = getEnclosingEvent(document, position, metadata);

                    for (const [name, vData] of Object.entries(metadata.variables)) {
                        const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
                        item.sortText = ' ' + name;
                        item.detail = `(Global: ${vData.type})${vData.optional ? ' (optional)' : ''}`;
                        items.push(item);
                    }
                    
                    if (eventName && metadata.events && metadata.events[eventName] && metadata.events[eventName].variables) {
                        for (const [name, vData] of Object.entries(metadata.events[eventName].variables)) {
                            const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
                            item.sortText = ' ' + name;
                            item.detail = `(Context: ${vData.type})${vData.optional ? ' (optional)' : ''}`;
                            items.push(item);
                        }
                    }
                    
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
                    const returnTypeStr = (data.type === 'List' && (data as any).item_type) ? `List<${(data as any).item_type}>` : data.type;
                    item.detail = `(Function) ${name}(${params}) -> ${returnTypeStr}`;
                    item.documentation = new vscode.MarkdownString(data.description);
                    
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

                    const wordRange = document.getWordRangeAtPosition(position, /[\$a-zA-Z0-9_]+/);
                    if (!wordRange) return null;

                    const word = document.getText(wordRange);
                    const eventName = getEnclosingEvent(document, position, metadata);
                    
                    if (isInFxLangProgram(document, position)) {
                        const symbols = parseContext(document, position, metadata);
                    
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
                                const parentType = resolveType(parentChain, symbols, metadata, eventName);
                                
                                if (parentType) {
                                    const typeMembers = getTypeMembers(parentType, metadata);
                                    let memberData = typeMembers[lastMember];
                                    let isPotentialMove = false;
                                    
                                    if (!memberData && parentType === 'Effect') {
                                        const moveMembers = metadata.type_members['ActiveMove'];
                                        if (moveMembers && moveMembers[lastMember]) {
                                            memberData = moveMembers[lastMember];
                                            isPotentialMove = true;
                                        }
                                    }
                                    
                                    if (memberData) {
                                        const origin = isPotentialMove ? `Potential ActiveMove` : parentType;
                                        return new vscode.Hover(new vscode.MarkdownString(`**Member \`${lastMember}\`** of \`${origin}\`\n\nType: \`${memberData.type}\`\n\n${memberData.description}`));
                                    }
                                }
                                
                                if (metadata.variable_members[lastMember]) {
                                    const memberData = metadata.variable_members[lastMember];
                                    return new vscode.Hover(new vscode.MarkdownString(`**Member \`${lastMember}\`** (Global)\n\nType: \`${memberData.type}\`\n\n${memberData.description}`));
                                }
                                // Check for pseudo-methods
                                if (parentType && metadata.functions[lastMember]) {
                                    const fnData = metadata.functions[lastMember];
                                    const firstParam = fnData.parameters[0];
                                    if (firstParam) {
                                        const pType = firstParam.type;
                                        const isMatch = pType === parentType || pType === 'Any' ||
                                            (pType === 'Object' && (parentType === 'BoostTable' || parentType === 'StatTable' || parentType === 'EffectState')) ||
                                            (pType === 'Fraction' && parentType === 'UFraction') ||
                                            (pType === 'UFraction' && parentType === 'Fraction');
                                        
                                        if (isMatch) {
                                            const params = fnData.parameters.map(p => p.optional ? `[${p.name}: ${p.type}]` : `${p.name}: ${p.type}`).join(', ');
                                            const paramDetails = fnData.parameters.map(p => `* \`${p.name}\`: \`${p.type}\`${p.optional ? ' (optional)' : ''} - ${p.description}`).join('\n');
                                            const flagDetails = (fnData.flags || []).map(f => `* \`${f.name}\` - ${f.description}`).join('\n');
                                            
                                            const hoverText = new vscode.MarkdownString();
                                            hoverText.appendMarkdown(`**Pseudo-Method \`${lastMember}\`**\n\n`);
                                            const returnTypeStr = (fnData.type === 'List' && (fnData as any).item_type) ? `List<${(fnData as any).item_type}>` : fnData.type;
                                            hoverText.appendCodeblock(`${lastMember}(${params}) -> ${returnTypeStr}`, 'fxlang');
                                            hoverText.appendMarkdown(`\n\n${fnData.description}`);
                                            if (paramDetails) hoverText.appendMarkdown(`\n\n**Parameters:**\n${paramDetails}`);
                                            if (flagDetails) hoverText.appendMarkdown(`\n\n**Flags:**\n${flagDetails}`);
                                            
                                            return new vscode.Hover(hoverText);
                                        }
                                    }
                                }
                                
                                return undefined;
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
                        const returnTypeStr = (data.type === 'List' && (data as any).item_type) ? `List<${(data as any).item_type}>` : data.type;
                        hoverText.appendCodeblock(`${word2}(${params}) -> ${returnTypeStr}`, 'fxlang');
                        hoverText.appendMarkdown(`\n\n${data.description}`);
                        if (paramDetails) {
                            hoverText.appendMarkdown(`\n\n**Parameters:**\n${paramDetails}`);
                        }
                        if (flagDetails) {
                            hoverText.appendMarkdown(`\n\n**Flags:**\n${flagDetails}`);
                        }
                        
                        return new vscode.Hover(hoverText);
                        }
                    } else {
                        const eventKey = resolveEventName(word, metadata);
                        if (eventKey && metadata.events[eventKey]) {
                        const eventData = metadata.events[eventKey];
                        const hoverText = new vscode.MarkdownString();
                        hoverText.appendMarkdown(`**Event \`${word}\`**\n\n`);
                        hoverText.appendMarkdown(`${eventData.description}`);
                        return new vscode.Hover(hoverText);
                        }
                    }
                } catch (err) {
                    try {
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
                            const primaryName = baseName;
                            const item = new vscode.CompletionItem(primaryName, vscode.CompletionItemKind.Event);
                            item.sortText = ' ' + primaryName;
                            item.filterText = `"${primaryName}`;
                            item.range = quoteRange;
                            item.documentation = new vscode.MarkdownString(data.description);
                            item.insertText = new vscode.SnippetString(`"${primaryName}": [\n\t$0\n],`);
                            items.push(item);
                        } else {
                            const primaryName = 'on_' + baseName;
                            const item = new vscode.CompletionItem(primaryName, vscode.CompletionItemKind.Event);
                            item.sortText = ' ' + primaryName;
                            item.filterText = `"${primaryName}`;
                            item.range = quoteRange;
                            item.documentation = new vscode.MarkdownString(data.description);
                            item.insertText = new vscode.SnippetString(`"${primaryName}": [\n\t$0\n],`);
                            items.push(item);
                            
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

    const decorationType = vscode.window.createTextEditorDecorationType({
        gutterIconPath: vscode.Uri.file(path.join(context.extensionPath, 'media', 'fxlang-event.svg')),
        gutterIconSize: 'contain'
    });

    function updateDecorations() {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;

        let isFxLang = editor.document.languageId === 'fxlang';
        if (!isFxLang) {
            const text = editor.document.getText();
            if (text.includes('"program"') || text.includes('"callbacks"')) {
                isFxLang = true;
            }
        }
        if (!isFxLang) {
            editor.setDecorations(decorationType, []);
            return;
        }

        const ranges: vscode.Range[] = [];
        for (let i = 0; i < editor.document.lineCount; i++) {
            const line = editor.document.lineAt(i).text.trim();
            const match = line.match(/^"([a-zA-Z0-9_]+)"\s*:\s*[\[{]/);
            if (match) {
                const rawName = match[1];
                if (resolveEventName(rawName, metadata)) {
                    const range = new vscode.Range(i, 0, i, 0);
                    ranges.push(range);
                }
            }
        }
        editor.setDecorations(decorationType, ranges);
    }

    vscode.window.onDidChangeActiveTextEditor(editor => {
        if (editor) updateDecorations();
    }, null, context.subscriptions);

    vscode.workspace.onDidChangeTextDocument(event => {
        const editor = vscode.window.activeTextEditor;
        if (editor && event.document === editor.document) {
            updateDecorations();
        }
    }, null, context.subscriptions);

    updateDecorations();

    const codeLensProvider = vscode.languages.registerCodeLensProvider(
        languages,
        {
            provideCodeLenses(document: vscode.TextDocument) {
                const lenses: vscode.CodeLens[] = [];
                for (let i = 0; i < document.lineCount; i++) {
                    const line = document.lineAt(i);
                    if (line.text.includes('"callbacks"')) {
                        const position = new vscode.Position(i, line.text.indexOf('"callbacks"'));
                        const range = new vscode.Range(position, position);
                        lenses.push(new vscode.CodeLens(range, {
                            title: "⚡ fxlang active",
                            command: ""
                        }));
                    }
                }
                return lenses;
            }
        }
    );

    context.subscriptions.push(completionProvider, hoverProvider, eventProvider, codeLensProvider);
}

export function deactivate() {}
