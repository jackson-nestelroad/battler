import * as vscode from 'vscode';

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

export function getPrefixAtPosition(document: vscode.TextDocument, position: vscode.Position): string {
    const line = document.lineAt(position).text;
    const prefix = line.substring(0, position.character);
    const match = prefix.match(/(\$[a-zA-Z0-9_.]*)$/);
    return match ? match[1] : '';
}
