import * as vscode from 'vscode';
import * as path from 'path';
import { Metadata, FxLangParseResult, SymbolTable } from './types';
import { FxLangParser } from './Parser';
import { TypeEngine } from './TypeEngine';
import { DocumentContextManager } from './DocumentContext';
import { SymbolEngine } from './SymbolEngine';
import { getChainAtPosition } from './utils';

export function activate(context: vscode.ExtensionContext) {
    const outputChannel = vscode.window.createOutputChannel("FxLang");
    context.subscriptions.push(outputChannel);
    outputChannel.appendLine("FxLang extension activated");

    const metadataPath = path.join(context.extensionPath, 'metadata.json');
    let metadata: Metadata = {
        variables: {},
        variable_members: {},
        type_members: {},
        functions: {},
        events: {},
        common_flags: []
    };

    // Engines
    const parser = new FxLangParser(metadata);
    const typeEngine = new TypeEngine(metadata);
    const docContext = new DocumentContextManager(metadata);
    const symbolEngine = new SymbolEngine(metadata, parser, typeEngine);

    async function loadMetadata() {
        try {
            const data = await vscode.workspace.fs.readFile(vscode.Uri.file(metadataPath));
            metadata = JSON.parse(Buffer.from(data).toString('utf8'));
            
            // Update all engines
            parser.updateMetadata(metadata);
            FxLangParser.clearResolutionCache();
            typeEngine.updateMetadata(metadata);
            docContext.updateMetadata(metadata);
            symbolEngine.updateMetadata(metadata);
            
            updateDecorations();
        } catch (e) {
            console.error('Failed to load fxlang metadata', e);
        }
    }

    loadMetadata();

    const watcher = vscode.workspace.createFileSystemWatcher(metadataPath);
    watcher.onDidChange(() => loadMetadata());
    context.subscriptions.push(watcher);

    const languages = ['fxlang', 'json', 'jsonc'];

    // Register Providers
    context.subscriptions.push(
        vscode.languages.registerCompletionItemProvider(languages, new FxCompletionItemProvider(typeEngine, docContext, symbolEngine), '.', '$', '"', '('),
        vscode.languages.registerHoverProvider(languages, new FxHoverProvider(typeEngine, docContext, symbolEngine, parser)),
        vscode.languages.registerCompletionItemProvider(languages, new FxEventCompletionItemProvider(typeEngine, docContext, parser), '"'),
        vscode.languages.registerCodeLensProvider(languages, new FxCodeLensProvider(docContext)),
        vscode.languages.registerDocumentSymbolProvider(languages, new FxDocumentSymbolProvider(docContext, parser))
    );

    // Decorations
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
        if (!editor || !docContext.isRelevant(editor.document)) return;

        const visibleRanges = editor.visibleRanges;
        const gutterRanges: vscode.Range[] = [];
        const inlineDecorations: vscode.DecorationOptions[] = [];

        const { blocks, mappings } = docContext.getContext(editor.document);

        for (const block of blocks) {
            const isVisible = visibleRanges.some(vr => 
                block.startLine >= vr.start.line - 10 && block.startLine <= vr.end.line + 10
            );
            if (isVisible) {
                gutterRanges.push(new vscode.Range(block.startLine, 0, block.startLine, 0));
            }
        }

        for (const m of mappings) {
            const isVisible = visibleRanges.some(vr => 
                m.documentLine >= vr.start.line - 10 && m.documentLine <= vr.end.line + 10
            );
            if (isVisible) {
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
        }

        const showOverviewRuler = vscode.workspace.getConfiguration('fxlang').get<boolean>('showOverviewRuler', false);
        if (showOverviewRuler) {
            editor.setDecorations(decorationType, gutterRanges);
            editor.setDecorations(decorationTypeWithoutRuler, []);
        } else {
            editor.setDecorations(decorationType, []);
            editor.setDecorations(decorationTypeWithoutRuler, gutterRanges);
        }
        editor.setDecorations(marginDecorationType, []);

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

    let statusBarVisible = false;
    function updateStatusBar() {
        const editor = vscode.window.activeTextEditor;
        if (!editor || !docContext.isRelevant(editor.document)) {
            if (statusBarVisible) {
                statusBarItem.hide();
                statusBarVisible = false;
            }
            return;
        }

        const position = editor.selection.active;
        let activeLineIndex = -1;

        const { mappings } = docContext.getContext(editor.document);

        for (const m of mappings) {
            if (m.documentLine === position.line) {
                if (activeLineIndex === -1 || position.character >= m.charStart) {
                    activeLineIndex = m.lineIndex;
                }
            }
        }

        if (activeLineIndex !== -1) {
            const text = `$(list-numbered) fxlang line: ${activeLineIndex}`;
            if (statusBarItem.text !== text) statusBarItem.text = text;
            if (!statusBarVisible) {
                statusBarItem.show();
                statusBarVisible = true;
            }
        } else if (statusBarVisible) {
            statusBarItem.hide();
            statusBarVisible = false;
        }
    }

    vscode.commands.registerCommand('fxlang.toggleLineNumbers', () => {
        sessionShowLineNumbers = !(sessionShowLineNumbers ?? vscode.workspace.getConfiguration('fxlang').get<boolean>('showLineNumbers', false));
        updateDecorations();
        vscode.window.showInformationMessage(`fxlang line numbers: ${sessionShowLineNumbers ? 'on' : 'off'}`);
    });

    let decorationTimeout: NodeJS.Timeout | undefined;
    function debouncedUpdateDecorations() {
        if (decorationTimeout) clearTimeout(decorationTimeout);
        decorationTimeout = setTimeout(() => {
            updateDecorations();
            decorationTimeout = undefined;
        }, 100);
    }

    let statusTimeout: NodeJS.Timeout | undefined;
    function debouncedUpdateStatusBar() {
        if (statusTimeout) clearTimeout(statusTimeout);
        statusTimeout = setTimeout(() => {
            updateStatusBar();
            statusTimeout = undefined;
        }, 50);
    }

    // Event Listeners
    vscode.window.onDidChangeActiveTextEditor(() => {
        updateDecorations();
        updateStatusBar();
    }, null, context.subscriptions);

    vscode.window.onDidChangeTextEditorVisibleRanges(e => {
        if (e.textEditor === vscode.window.activeTextEditor) debouncedUpdateDecorations();
    }, null, context.subscriptions);

    vscode.workspace.onDidChangeConfiguration(e => {
        if (e.affectsConfiguration('fxlang.showOverviewRuler') || e.affectsConfiguration('fxlang.showLineNumbers')) updateDecorations();
    }, null, context.subscriptions);

    vscode.window.onDidChangeTextEditorSelection(event => {
        if (event.textEditor === vscode.window.activeTextEditor) debouncedUpdateStatusBar();
    }, null, context.subscriptions);

    vscode.workspace.onDidCloseTextDocument(doc => {
        docContext.clear(doc.uri.toString());
    }, null, context.subscriptions);

    vscode.workspace.onDidChangeTextDocument(event => {
        if (event.document === vscode.window.activeTextEditor?.document) {
            debouncedUpdateDecorations();
            debouncedUpdateStatusBar();
        }
    }, null, context.subscriptions);

    updateDecorations();
    updateStatusBar();
}

export function deactivate() {}

class FxCompletionItemProvider implements vscode.CompletionItemProvider {
    constructor(private typeEngine: TypeEngine, private docContext: DocumentContextManager, private symbolEngine: SymbolEngine) {}

    public provideCompletionItems(document: vscode.TextDocument, position: vscode.Position): vscode.ProviderResult<vscode.CompletionItem[]> {
        if (this.docContext.getEnclosingBlockType(document, position) !== 'array') return undefined;

        const symbols = this.symbolEngine.parseContext(document, position, true);
        const linePrefix = document.lineAt(position).text.substring(0, position.character);
        
        if (linePrefix.match(/\.([\w]*)$/)) {
            return this.getMemberCompletions(document, position, symbols);
        }

        if (linePrefix.match(/\$([\w]*)$/)) {
            return this.getVariableCompletions(document, position, symbols);
        }

        return this.getGlobalCompletions(document, position);
    }

    private getMemberCompletions(document: vscode.TextDocument, position: vscode.Position, symbols: SymbolTable): vscode.CompletionItem[] {
        const chain = getChainAtPosition(document, position);
        const parseResult = this.docContext.getContext(document);
        const block = parseResult.blocks.find(b => position.line >= b.startLine && position.line <= b.endLine);
        const type = this.typeEngine.resolveChainType(chain, symbols, block?.eventName);
        
        const varName = chain.join('.');
        const wordRange = document.getWordRangeAtPosition(position, /[\$a-zA-Z0-9_.]+/);
        const items: vscode.CompletionItem[] = [];

        if (type) {
            const typeMembers = this.typeEngine.getTypeMembers(type);
            for (const [name, data] of Object.entries(typeMembers)) {
                const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Field);
                item.documentation = new vscode.MarkdownString(data.description);
                item.detail = `(Member of ${type} -> ${this.typeEngine.getDisplayType(data.type, data.item_type)})`;
                item.sortText = '0_' + name;
                item.filterText = `${varName}.${name}`;
                if (wordRange) item.range = wordRange;
                item.insertText = `${varName}.${name}`;
                items.push(item);
            }

            // Pseudo-methods
            for (const [name, data] of Object.entries(this.typeEngine.metadata.functions)) {
                if (data.parameters[0] && this.typeEngine.areTypesCompatible(type, data.parameters[0].type)) {
                    const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Method);
                    item.sortText = '3_' + name;
                    item.documentation = new vscode.MarkdownString(data.description);
                    item.filterText = `${varName}.${name}`;
                    const snippetParams = data.parameters.slice(1).map((p, i) => `\${${i + 1}:${p.name}}`).join(' ');
                    item.insertText = new vscode.SnippetString(`${name}: ${varName.replace(/\$/g, '\\$')}${snippetParams ? ' ' + snippetParams : ''}`);
                    if (wordRange) item.range = wordRange;
                    items.push(item);
                }
            }
        }

        // Global variable members
        for (const [name, data] of Object.entries(this.typeEngine.metadata.variable_members)) {
            const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Field);
            item.documentation = new vscode.MarkdownString(data.description);
            item.detail = `(Global -> ${data.type})`;
            item.sortText = '2_' + name;
            item.filterText = `${varName}.${name}`;
            if (wordRange) item.range = wordRange;
            item.insertText = `${varName}.${name}`;
            items.push(item);
        }

        return items;
    }

    private getVariableCompletions(document: vscode.TextDocument, position: vscode.Position, symbols: SymbolTable): vscode.CompletionItem[] {
        const items: vscode.CompletionItem[] = [];
        const parseResult = this.docContext.getContext(document);
        const block = parseResult.blocks.find(b => position.line >= b.startLine && position.line <= b.endLine);
        const eventName = block?.eventName;

        for (const [name, vData] of Object.entries(this.typeEngine.metadata.variables)) {
            const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
            item.detail = `(Global: ${vData.type})`;
            items.push(item);
        }
        
        if (eventName && this.typeEngine.metadata.events?.[eventName]?.variables) {
            for (const [name, vData] of Object.entries(this.typeEngine.metadata.events[eventName].variables)) {
                const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
                item.detail = `(Context: ${vData.type})`;
                items.push(item);
            }
        }
        
        if (this.typeEngine.metadata.events?.[eventName || '']?.allows_custom_input_vars) {
            for (const name of this.symbolEngine.getCustomVariables(document, position)) {
                const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
                item.detail = `(Custom Parameter)`;
                items.push(item);
            }
        }
        
        for (const [name, type] of Object.entries(symbols)) {
            const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Variable);
            item.detail = `(Local: ${type})`;
            items.push(item);
        }
        
        return items;
    }

    private getGlobalCompletions(document: vscode.TextDocument, position: vscode.Position): vscode.CompletionItem[] {
        const items: vscode.CompletionItem[] = [];
        const wordRange = document.getWordRangeAtPosition(position, /[a-zA-Z0-9_]+/) || new vscode.Range(position, position);
        
        for (const [name, data] of Object.entries(this.typeEngine.metadata.functions)) {
            const item = new vscode.CompletionItem(name, vscode.CompletionItemKind.Function);
            item.range = wordRange;
            item.detail = `(Function) ${name}`;
            item.documentation = new vscode.MarkdownString(data.description);
            const snippetParams = data.parameters.map((p, i) => `\${${i + 1}:${p.name}}`).join(' ');
            item.insertText = new vscode.SnippetString(`${name}${snippetParams ? ': ' + snippetParams : ''}`);
            items.push(item);
        }
        
        ['if', 'else', 'foreach', 'in', 'return', 'break', 'continue', 'and', 'or', 'has', 'hasany', 'func_call', 'expr', 'str'].forEach(kw => {
            const item = new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword);
            item.range = wordRange;
            if (kw === 'func_call') {
                item.insertText = new vscode.SnippetString('func_call($0)');
                item.command = { command: 'editor.action.triggerSuggest', title: 'Suggest' };
            }
            items.push(item);
        });

        ['true', 'false', 'undefined', 'stop', ...(this.typeEngine.metadata.common_flags || [])].forEach(c => {
            const item = new vscode.CompletionItem(c, vscode.CompletionItemKind.Constant);
            item.range = wordRange;
            items.push(item);
        });

        return items;
    }
}

class FxHoverProvider implements vscode.HoverProvider {
    constructor(private typeEngine: TypeEngine, private docContext: DocumentContextManager, private symbolEngine: SymbolEngine, private parser: FxLangParser) {}

    public provideHover(document: vscode.TextDocument, position: vscode.Position): vscode.ProviderResult<vscode.Hover> {
        const wordRange = document.getWordRangeAtPosition(position, /[\$a-zA-Z0-9_]+/);
        if (!wordRange) return null;

        const word = document.getText(wordRange);
        
        if (this.docContext.isInFxLangProgram(document, position)) {
            const parseResult = this.docContext.getContext(document);
            const block = parseResult.blocks.find(b => position.line >= b.startLine && position.line <= b.endLine);
            const symbols = this.symbolEngine.parseContext(document, position);
            
            if (word.startsWith('$')) {
                const varName = word.substring(1);
                const varData = this.typeEngine.getVariableData(varName, block?.eventName);
                const type = symbols[varName] || (varData ? this.typeEngine.getDisplayType(varData.type, varData.item_type) : undefined);
                if (type) {
                    return new vscode.Hover(new vscode.MarkdownString(`**Variable \`${word}\`**\n\nType: \`${type}\``));
                }
            } else {
                const fullRange = document.getWordRangeAtPosition(position, /[\$a-zA-Z0-9_.]+/);
                if (fullRange) {
                    const chain = document.getText(fullRange).split('.');
                    const wordIndex = chain.indexOf(word);
                    if (wordIndex > 0) {
                        const parentType = this.typeEngine.resolveChainType(chain.slice(0, wordIndex), symbols, block?.eventName);
                        if (parentType) {
                            const member = this.typeEngine.getTypeMembers(parentType)[word];
                            if (member) {
                                return new vscode.Hover(new vscode.MarkdownString(`**Member \`${word}\`** of \`${parentType}\`\n\nType: \`${this.typeEngine.getDisplayType(member.type, member.item_type)}\`\n\n${member.description}`));
                            }
                        }
                    }
                }
                const func = this.typeEngine.metadata.functions[word];
                if (func) {
                    return new vscode.Hover(new vscode.MarkdownString(`**Function \`${word}\`**\n\n${func.description}`));
                }
            }
        } else {
            const eventKey = this.parser.resolveEventName(word);
            if (eventKey && this.typeEngine.metadata.events?.[eventKey]) {
                return new vscode.Hover(new vscode.MarkdownString(`**Event \`${word}\`**\n\n${this.typeEngine.metadata.events[eventKey].description}`));
            }
        }
        return null;
    }
}

class FxEventCompletionItemProvider implements vscode.CompletionItemProvider {
    constructor(private typeEngine: TypeEngine, private docContext: DocumentContextManager, private parser: FxLangParser) {}

    public provideCompletionItems(document: vscode.TextDocument, position: vscode.Position): vscode.ProviderResult<vscode.CompletionItem[]> {
        if (this.docContext.getEnclosingBlockType(document, position) !== 'object') return undefined;

        const line = document.lineAt(position.line).text;
        let insideCallbacks = false;
        let insideEvent = false;
        let braceDepth = 0;
        
        for (let i = position.line; i >= 0; i--) {
            const l = document.lineAt(i).text.trim();
            if (l.includes('}')) braceDepth++;
            if (l.includes(']')) braceDepth++;
            if (l.includes('{')) braceDepth--;
            if (l.includes('[')) braceDepth--;
            
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

        const textBeforeCursor = line.substring(0, position.character);
        const quoteRange = document.getWordRangeAtPosition(position, /"[a-zA-Z0-9_]*"?/) || new vscode.Range(position, position);
        const items: vscode.CompletionItem[] = [];
        const metadata = this.typeEngine.metadata;
        
        for (const [baseName, data] of Object.entries(metadata.events || {})) {
            const primaryName = (baseName.startsWith('is_') || baseName.startsWith('suppress_')) ? baseName : 'on_' + baseName;
            const item = new vscode.CompletionItem(primaryName, vscode.CompletionItemKind.Event);
            item.range = quoteRange;
            item.filterText = `"${primaryName}`;
            item.insertText = new vscode.SnippetString(`"${primaryName}": [\n\t$0\n],`);
            item.documentation = new vscode.MarkdownString(data.description);
            items.push(item);
            
            if (primaryName.startsWith('on_')) {
                ['ally', 'any', 'field', 'foe', 'side', 'source'].forEach(mod => {
                    const modName = `on_${mod}_${baseName}`;
                    const modItem = new vscode.CompletionItem(modName, vscode.CompletionItemKind.Event);
                    modItem.range = quoteRange;
                    modItem.filterText = `"${modName}`;
                    modItem.insertText = new vscode.SnippetString(`"${modName}": [\n\t$0\n],`);
                    modItem.documentation = new vscode.MarkdownString(`*(Modifier: ${mod})*\n\n` + data.description);
                    items.push(modItem);
                });
            }
        }
        return items;
    }
}

class FxCodeLensProvider implements vscode.CodeLensProvider {
    constructor(private docContext: DocumentContextManager) {}
    public provideCodeLenses(document: vscode.TextDocument) {
        if (!this.docContext.isRelevant(document)) return [];
        const lenses: vscode.CodeLens[] = [];
        const text = document.getText();
        let index = text.indexOf('"callbacks"');
        while (index !== -1) {
            const pos = document.positionAt(index);
            lenses.push(new vscode.CodeLens(new vscode.Range(pos, pos), { title: "⚡ fxlang active", command: "" }));
            index = text.indexOf('"callbacks"', index + 1);
        }
        return lenses;
    }
}

class FxDocumentSymbolProvider implements vscode.DocumentSymbolProvider {
    constructor(private docContext: DocumentContextManager, private parser: FxLangParser) {}
    public provideDocumentSymbols(document: vscode.TextDocument) {
        const { blocks } = this.docContext.getContext(document);
        return blocks.map(block => {
            const name = block.eventName ? `on_${block.eventName}` : "fxlang program";
            const range = new vscode.Range(block.startLine, 0, block.endLine, 0);
            return new vscode.DocumentSymbol(name, "fxlang script", vscode.SymbolKind.Function, range, range);
        });
    }
}
