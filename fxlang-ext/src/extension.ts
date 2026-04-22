import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

interface Metadata {
    variable_members: Record<string, { description: string }>;
    type_members: Record<string, Record<string, { description: string }>>;
    functions: Record<string, { description: string }>;
    events: Record<string, { description: string }>;
}

export function activate(context: vscode.ExtensionContext) {
    const metadataPath = path.join(context.extensionPath, 'metadata.json');
    let metadata: Metadata = {
        variable_members: {},
        type_members: {},
        functions: {},
        events: {}
    };

    if (fs.existsSync(metadataPath)) {
        try {
            metadata = JSON.parse(fs.readFileSync(metadataPath, 'utf8'));
        } catch (e) {
            console.error('Failed to load fxlang metadata', e);
        }
    }

    // Completion Provider
    const completionProvider = vscode.languages.registerCompletionItemProvider(
        { language: 'fxlang' },
        {
            provideCompletionItems(document: vscode.TextDocument, position: vscode.Position) {
                const linePrefix = document.lineAt(position).text.substr(0, position.character);
                
                // Member completion (after .)
                if (linePrefix.endsWith('.')) {
                    const items: vscode.CompletionItem[] = [];
                    // Combine all known members for a generic experience
                    // A better version would track the type of the variable
                    for (const type of Object.keys(metadata.type_members)) {
                        for (const member of Object.keys(metadata.type_members[type])) {
                            const item = new vscode.CompletionItem(member, vscode.CompletionItemKind.Field);
                            item.documentation = new vscode.MarkdownString(metadata.type_members[type][member].description);
                            item.detail = `(Member of ${type})`;
                            items.push(item);
                        }
                    }
                    // Add global members
                    for (const member of Object.keys(metadata.variable_members)) {
                        const item = new vscode.CompletionItem(member, vscode.CompletionItemKind.Field);
                        item.documentation = new vscode.MarkdownString(metadata.variable_members[member].description);
                        items.push(item);
                    }
                    return items;
                }

                // Variable completion (after $)
                if (linePrefix.endsWith('$')) {
                    const items: vscode.CompletionItem[] = [];
                    const variables = ['$source', '$target', '$move', '$self', '$battle', '$user'];
                    for (const v of variables) {
                        items.push(new vscode.CompletionItem(v.substring(1), vscode.CompletionItemKind.Variable));
                    }
                    return items;
                }

                // Function completion (top level)
                const items: vscode.CompletionItem[] = [];
                for (const func of Object.keys(metadata.functions)) {
                    const item = new vscode.CompletionItem(func, vscode.CompletionItemKind.Function);
                    item.documentation = new vscode.MarkdownString(metadata.functions[func].description);
                    item.insertText = new vscode.SnippetString(`${func}: `);
                    items.push(item);
                }
                
                // Control keywords
                const keywords = ['if', 'else', 'elseif', 'foreach', 'in', 'return', 'break', 'continue'];
                for (const kw of keywords) {
                    items.push(new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword));
                }

                return items;
            }
        },
        '.', '$'
    );

    // Hover Provider
    const hoverProvider = vscode.languages.registerHoverProvider(
        { language: 'fxlang' },
        {
            provideHover(document: vscode.TextDocument, position: vscode.Position) {
                const range = document.getWordRangeAtPosition(position);
                const word = document.getText(range);

                if (!word) return;

                // Check functions
                if (metadata.functions[word]) {
                    return new vscode.Hover(new vscode.MarkdownString(metadata.functions[word].description));
                }

                // Check members
                for (const type of Object.keys(metadata.type_members)) {
                    if (metadata.type_members[type][word]) {
                        return new vscode.Hover(new vscode.MarkdownString(`**${type} Member**: ${metadata.type_members[type][word].description}`));
                    }
                }
                
                if (metadata.variable_members[word]) {
                    return new vscode.Hover(new vscode.MarkdownString(metadata.variable_members[word].description));
                }

                return null;
            }
        }
    );

    context.subscriptions.push(completionProvider, hoverProvider);
}

export function deactivate() {}
