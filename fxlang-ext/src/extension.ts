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
    getTypeMembers,
    parseFxLangDocument,
    getDisplayType,
    areTypesCompatible,
    EVENT_MODIFIERS,
    getVariableData,
    getCustomVariables
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
        new FxCompletionItemProvider(() => metadata),
        '.', '$', '"', '('
    );

    // Hover Provider
    const hoverProvider = vscode.languages.registerHoverProvider(
        languages,
        new FxHoverProvider(() => metadata)
    );

    // Event Callback Autocomplete
    const eventProvider = vscode.languages.registerCompletionItemProvider(
        languages,
        new FxEventCompletionItemProvider(() => metadata),
        '"'
    );

    const decorationType = vscode.window.createTextEditorDecorationType({
        gutterIconPath: vscode.Uri.file(path.join(context.extensionPath, 'media', 'fxlang-mono.svg')),
        gutterIconSize: 'contain',
        overviewRulerLane: vscode.OverviewRulerLane.Right,
        overviewRulerColor: '#f1c40f'
    });

    const decorationTypeWithoutRuler = vscode.window.createTextEditorDecorationType({
        gutterIconPath: vscode.Uri.file(path.join(context.extensionPath, 'media', 'fxlang-mono.svg')),
        gutterIconSize: 'contain'
    });

    const marginDecorationType = vscode.window.createTextEditorDecorationType({
        after: {
            margin: '0 0 0 2em',
            color: new vscode.ThemeColor('editorCodeLens.foreground'),
        }
    });

    const inlineDecorationType = vscode.window.createTextEditorDecorationType({
        before: {
            margin: '0 0.5em 0 0',
            color: new vscode.ThemeColor('editorCodeLens.foreground'),
        }
    });

    const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    context.subscriptions.push(statusBarItem);



    let sessionShowLineNumbers: boolean | undefined = undefined;

    function updateDecorations() {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;

        const gutterRanges: vscode.Range[] = [];
        const marginDecorations: vscode.DecorationOptions[] = [];
        const inlineDecorations: vscode.DecorationOptions[] = [];

        const { blocks, mappings } = parseFxLangDocument(editor.document, metadata);

        for (const block of blocks) {
            gutterRanges.push(new vscode.Range(block.startLine, 0, block.startLine, 0));
        }

        for (const m of mappings) {
            const startPos = new vscode.Position(m.documentLine, m.charStart + 1);
            inlineDecorations.push({
                range: new vscode.Range(startPos, startPos),
                renderOptions: {
                    before: {
                        contentText: `L${m.lineIndex}`,
                    }
                }
            });
        }

        const showOverviewRuler = vscode.workspace.getConfiguration('fxlang').get<boolean>('showOverviewRuler', false);
        if (showOverviewRuler) {
            editor.setDecorations(decorationType, gutterRanges);
            editor.setDecorations(decorationTypeWithoutRuler, []);
        } else {
            editor.setDecorations(decorationType, []);
            editor.setDecorations(decorationTypeWithoutRuler, gutterRanges);
        }
        editor.setDecorations(marginDecorationType, marginDecorations);

        let showLineNumbers = sessionShowLineNumbers;
        if (showLineNumbers === undefined) {
            showLineNumbers = vscode.workspace.getConfiguration('fxlang').get<boolean>('showLineNumbers', false);
        }
        if (!showLineNumbers) {
            editor.setDecorations(inlineDecorationType, []);
        } else {
            editor.setDecorations(inlineDecorationType, inlineDecorations);
        }
    }

    function updateStatusBar() {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            statusBarItem.hide();
            return;
        }

        const position = editor.selection.active;
        let activeLineIndex = -1;

        const { mappings } = parseFxLangDocument(editor.document, metadata);

        for (const m of mappings) {
            if (m.documentLine === position.line) {
                if (activeLineIndex === -1 || position.character >= m.charStart) {
                    activeLineIndex = m.lineIndex;
                }
            }
        }

        if (activeLineIndex !== -1) {
            statusBarItem.text = `$(list-numbered) fxlang line: ${activeLineIndex}`;
            statusBarItem.show();
        } else {
            statusBarItem.hide();
        }
    }

    vscode.commands.registerCommand('fxlang.toggleLineNumbers', () => {
        let current = sessionShowLineNumbers;
        if (current === undefined) {
            current = vscode.workspace.getConfiguration('fxlang').get<boolean>('showLineNumbers', false);
        }
        sessionShowLineNumbers = !current;
        updateDecorations();
        vscode.window.showInformationMessage(`fxlang line numbers: ${sessionShowLineNumbers ? 'on' : 'off'}`);
    });

    vscode.window.onDidChangeActiveTextEditor(editor => {
        if (editor) {
            updateDecorations();
            updateStatusBar();
        }
    }, null, context.subscriptions);

    vscode.workspace.onDidChangeConfiguration(e => {
        if (e.affectsConfiguration('fxlang.showOverviewRuler') || e.affectsConfiguration('fxlang.showLineNumbers')) {
            updateDecorations();
        }
    }, null, context.subscriptions);

    vscode.window.onDidChangeTextEditorSelection(event => {
        if (event.textEditor === vscode.window.activeTextEditor) {
            updateStatusBar();
        }
    }, null, context.subscriptions);

    vscode.workspace.onDidChangeTextDocument(event => {
        const editor = vscode.window.activeTextEditor;
        if (editor && event.document === editor.document) {
            updateDecorations();
            updateStatusBar();
        }
    }, null, context.subscriptions);

    updateDecorations();
    updateStatusBar();

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

class FxCompletionItemProvider implements vscode.CompletionItemProvider {
    constructor(private getMetadata: () => Metadata) {}

    public provideCompletionItems(document: vscode.TextDocument, position: vscode.Position): vscode.ProviderResult<vscode.CompletionItem[]> {
        const metadata = this.getMetadata();
        if (!isFxLangContext(document, position, metadata)) return undefined;
        if (getEnclosingBlockType(document, position) !== 'array') return undefined;

        const symbols = parseContext(document, position, metadata, true);
        const linePrefix = document.lineAt(position).text.substring(0, position.character);
        
        // Member completion (after .)
        const memberMatch = linePrefix.match(/\.([\w]*)$/);
        if (memberMatch) {
            const chain = getChainAtPosition(document, position);
            const eventName = getEnclosingEvent(document, position, metadata);
            const type = resolveType(chain, symbols, metadata, eventName);
            
            const varName = chain.join('.');
            const wordRange = document.getWordRangeAtPosition(position, /[\$a-zA-Z0-9_.]+/);
            
            let items: vscode.CompletionItem[] = [];
            if (type) {
                const typeMembers = getTypeMembers(type, metadata);
                items = Object.entries(typeMembers).map(([name, data]) => {
                    const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Field);
                    item.documentation = new vscode.MarkdownString(data.description);
                    
                    const isMoveOnly = (data as any).only_applicable_to_move;
                    const memberTypeStr = getDisplayType(data.type, (data as any).item_type);
                    item.detail = isMoveOnly 
                        ? `(Move Member of ${type} -> ${memberTypeStr})` 
                        : `(Member of ${type} -> ${memberTypeStr})`;
                        
                    item.sortText = '0_' + name;
                    item.filterText = `${varName}.${name}`;
                    if (wordRange) item.range = wordRange;
                    item.insertText = `${varName}.${name}`;
                    return item;
                });
            }

            // Add global variable members (shared across types, e.g., is_undefined)
            const globalItems = Object.entries(metadata.variable_members).map(([name, data]) => {
                const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Field);
                item.documentation = new vscode.MarkdownString(data.description);
                item.detail = `(Global -> ${data.type})`;
                item.sortText = '2_' + name;
                item.filterText = `${varName}.${name}`;
                if (wordRange) item.range = wordRange;
                item.insertText = `${varName}.${name}`;
                return item;
            });
            items.push(...globalItems);

            // Add functions that take this type as an optional first parameter (pseudo-methods)
            if (type) {
                const varName = chain.join('.');
                const types = type.split(' | ');
                for (const [name, data] of Object.entries(metadata.functions)) {
                    const firstParam = data.parameters[0];
                    if (firstParam) {
                        const pType = firstParam.type;
                        const isMatch = areTypesCompatible(type, pType);
                        
                        if (isMatch) {
                        const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Method);
                        item.sortText = '3_' + name;
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
                        const returnTypeStr = getDisplayType(data.type, (data as any).item_type);
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
            
            const eventMeta = eventName && metadata.events ? metadata.events[eventName] : undefined;
            if (eventMeta && eventMeta.allows_custom_input_vars) {
                const customVars = getCustomVariables(document, position);
                for (const name of customVars) {
                    const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
                    item.sortText = ' ' + name;
                    item.detail = `(Custom Parameter: unknown)`;
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
            const returnTypeStr = getDisplayType(data.type, (data as any).item_type);
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
}

class FxHoverProvider implements vscode.HoverProvider {
    constructor(private getMetadata: () => Metadata) {}

    public provideHover(document: vscode.TextDocument, position: vscode.Position): vscode.ProviderResult<vscode.Hover> {
        const metadata = this.getMetadata();
        try {
            if (!isFxLangContext(document, position, metadata)) return undefined;

            const wordRange = document.getWordRangeAtPosition(position, /[\$a-zA-Z0-9_]+/);
            if (!wordRange) return null;

            const word = document.getText(wordRange);
            const eventName = getEnclosingEvent(document, position, metadata);
            
            if (isInFxLangProgram(document, position, metadata)) {
                const symbols = parseContext(document, position, metadata);
            
                if (word.startsWith('$')) {
                    const varName = word.substring(1);
                    let type = symbols[varName];
                    let origin = 'Local';
                    let optional = false;
                    
                    if (!type) {
                        const varData = getVariableData(varName, metadata, eventName);
                        if (varData) {
                            type = getDisplayType(varData.type, varData.item_type);
                            optional = varData.optional;
                            origin = varData.origin;
                        }
                    }
                    
                    if (!type) {
                        const eventMeta = eventName && metadata.events ? metadata.events[eventName] : undefined;
                        if (eventMeta && eventMeta.allows_custom_input_vars) {
                            const customVars = getCustomVariables(document, position);
                            if (customVars.includes(varName)) {
                                type = 'unknown';
                                origin = 'Custom Parameter';
                            }
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
                                
                                if (memberData) {
                                    const isMoveOnly = (memberData as any).only_applicable_to_move;
                                    const isActiveMoveOnly = (memberData as any).only_applicable_to_active_move;
                                    let markdownText = `**Member \`${lastMember}\`** of \`${parentType}\`\n\n`;
                                    if (isActiveMoveOnly && parentType !== 'ActiveMove') {
                                        markdownText = `**Active Move Member \`${lastMember}\`** of \`${parentType}\` *(only applicable if \`ActiveMove\`)*\n\n`;
                                    } else if (isMoveOnly && parentType !== 'Move') {
                                        markdownText = `**Move Member \`${lastMember}\`** of \`${parentType}\` *(only applicable if \`Move\`)*\n\n`;
                                    }
                                    const memberTypeStr = getDisplayType(memberData.type, (memberData as any).item_type);
                                    markdownText += `Type: \`${memberTypeStr}\`\n\n${memberData.description}`;
                                    
                                    return new vscode.Hover(new vscode.MarkdownString(markdownText));
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
                                    const isMatch = areTypesCompatible(parentType, pType);
                                    
                                    if (isMatch) {
                                        const params = fnData.parameters.map(p => p.optional ? `[${p.name}: ${p.type}]` : `${p.name}: ${p.type}`).join(', ');
                                        const paramDetails = fnData.parameters.map(p => `* \`${p.name}\`: \`${p.type}\`${p.optional ? ' (optional)' : ''} - ${p.description}`).join('\n');
                                        const flagDetails = (fnData.flags || []).map(f => `* \`$(f.name)\` - ${f.description}`).join('\n');
                                        
                                        const hoverText = new vscode.MarkdownString();
                                        hoverText.appendMarkdown(`**Pseudo-Method \`${lastMember}\`**\n\n`);
                                        const returnTypeStr = getDisplayType(fnData.type, (fnData as any).item_type);
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
                
                const constants = ['true', 'false', 'undefined', 'stop'];
                if (constants.includes(word2)) {
                    const hoverText = new vscode.MarkdownString();
                    hoverText.appendMarkdown(`**Literal \`${word2}\`**`);
                    return new vscode.Hover(hoverText);
                }
                
                if (metadata.common_flags && metadata.common_flags.includes(word2)) {
                    const hoverText = new vscode.MarkdownString();
                    hoverText.appendMarkdown(`**Common Flag \`${word2}\`**`);
                    return new vscode.Hover(hoverText);
                }
                
                if (metadata.functions[word2]) {
                    const data = metadata.functions[word2];
                    const params = data.parameters.map(p => p.optional ? `[${p.name}: ${p.type}]` : `${p.name}: ${p.type}`).join(', ');
                    const paramDetails = data.parameters.map(p => `* \`${p.name}\`: \`${p.type}\`${p.optional ? ' (optional)' : ''} - ${p.description}`).join('\n');
                    const flagDetails = (data.flags || []).map(f => `* \`${f.name}\` - ${f.description}`).join('\n');
                    
                    const hoverText = new vscode.MarkdownString();
                    hoverText.appendMarkdown(`**Function \`${word2}\`**\n\n`);
                    const returnTypeStr = getDisplayType(data.type, (data as any).item_type);
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

class FxEventCompletionItemProvider implements vscode.CompletionItemProvider {
    constructor(private getMetadata: () => Metadata) {}

    public provideCompletionItems(document: vscode.TextDocument, position: vscode.Position): vscode.ProviderResult<vscode.CompletionItem[]> {
        const metadata = this.getMetadata();
        const line = document.lineAt(position.line).text;
        let insideCallbacks = false;
        let insideEvent = false;
        let braceDepth = 0;
        
        for (let i = position.line; i >= 0; i--) {
            const l = document.lineAt(i).text.trim();
            
            if (l.includes('}')) {
                braceDepth++;
            }
            if (l.includes(']')) {
                braceDepth++;
            }
            
            if (l.includes('{')) {
                braceDepth--;
            }
            if (l.includes('[')) {
                braceDepth--;
            }
            
            if (l.match(/^"callbacks"\s*:\s*\{/)) {
                insideCallbacks = true;
                break;
            }
            
            if (braceDepth < 0) {
                const keyMatch = l.match(/^"([a-zA-Z0-9_]+)"\s*:\s*[\{\[]/);
                if (keyMatch && keyMatch[1] !== 'callbacks') {
                    insideEvent = true;
                    break;
                }
                braceDepth = 0;
            }
        }
        
        if (!insideCallbacks || insideEvent) return undefined;
        if (getEnclosingBlockType(document, position) !== 'object') return undefined;
        
        const textBeforeCursor = line.substring(0, position.character);
        if (textBeforeCursor.match(/^\s*"/)) {
            const quoteRange = document.getWordRangeAtPosition(position, /"[a-zA-Z0-9_]*"?/) || new vscode.Range(position, position);
            const items: vscode.CompletionItem[] = [];
            const modifiers = EVENT_MODIFIERS;
            
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
}
