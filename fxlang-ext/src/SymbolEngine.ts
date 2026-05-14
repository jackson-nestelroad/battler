import * as vscode from 'vscode';
import { Metadata, SymbolTable } from './types';
import { FxLangParser } from './Parser';
import { TypeEngine } from './TypeEngine';

export class SymbolEngine {
    private cache = new Map<string, { version: number, blockStartLine: number, symbols: SymbolTable }>();

    constructor(private metadata: Metadata, private parser: FxLangParser, private typeEngine: TypeEngine) {}

    public updateMetadata(metadata: Metadata) {
        this.metadata = metadata;
        this.cache.clear();
    }

    public parseContext(document: vscode.TextDocument, position: vscode.Position, parseUpToCursor = false): SymbolTable {
        const uri = document.uri.toString();
        const version = document.version;

        let blockStartLine = -1;
        for (let i = position.line; i >= Math.max(0, position.line - 500); i--) {
            const line = document.lineAt(i).text;
            if (line.match(/"[a-z0-9_]+"\s*:\s*\[/i)) {
                blockStartLine = i;
                break;
            }
        }
        
        if (blockStartLine === -1) return {};

        if (!parseUpToCursor) {
            const cached = this.cache.get(uri);
            if (cached && cached.version === version && cached.blockStartLine === blockStartLine) {
                return cached.symbols;
            }
        }

        const symbols: SymbolTable = {};
        const eventName = this.parser.resolveEventName(this.findRawEventName(document, position));

        for (let i = blockStartLine; i <= position.line; i++) {
            let line = document.lineAt(i).text.trim();
            if (!line.includes('$') && !line.includes('foreach')) continue;

            line = line.replace(/^"/, '').replace(/",?$/, '');
            
            const assignMatch = line.match(/(?:set\s+)?(\$[a-zA-Z0-9_]+)\s*=\s*(.*)/);
            if (assignMatch) {
                const varName = assignMatch[1].substring(1);
                let expression = assignMatch[2].trim();
                
                if (parseUpToCursor && i === position.line) {
                    const cursorInLine = position.character - (document.lineAt(i).text.length - line.length);
                    expression = expression.substring(0, cursorInLine).trim();
                }

                const type = this.typeEngine.inferType(expression, symbols, eventName) || 'unknown';
                if (!this.typeEngine.getVariableData(varName, eventName) && !symbols[varName]) {
                    symbols[varName] = type;
                }
            }
            
            const foreachMatch = line.match(/foreach\s+(\$[a-zA-Z0-9_]+)\s+in\s*(.*)/);
            if (foreachMatch) {
                const varName = foreachMatch[1].substring(1);
                let expression = foreachMatch[2].replace(/\s*:\s*$/, '').trim();
                const listType = this.typeEngine.inferType(expression, symbols, eventName);
                const innerType = this.typeEngine.unwrapListType(listType);
                
                if (!this.typeEngine.getVariableData(varName, eventName) && !symbols[varName]) {
                    symbols[varName] = innerType;
                }
            }
        }
        
        if (!parseUpToCursor) {
            this.cache.set(uri, { version, blockStartLine, symbols });
        }
        
        return symbols;
    }

    private findRawEventName(document: vscode.TextDocument, position: vscode.Position): string {
        for (let i = position.line; i >= Math.max(0, position.line - 500); i--) {
            const line = document.lineAt(i).text.trim();
            const match = line.match(/^"([a-zA-Z0-9_]+)"\s*:\s*[\[{]/);
            if (match && match[1] !== 'program' && match[1] !== 'metadata') {
                return match[1];
            }
            if (line.match(/^"callbacks"\s*:\s*\{/)) break;
        }
        return '';
    }

    public getCustomVariables(document: vscode.TextDocument, position: vscode.Position): string[] {
        const params: string[] = [];
        for (let i = position.line; i >= 0; i--) {
            const line = document.lineAt(i).text.trim();
            if (line.includes('"parameters"')) {
                const match = line.match(/"parameters"\s*:\s*\[(.*?)\]/);
                if (match) {
                    return match[1].split(',').map(p => p.trim().replace(/^"/, '').replace(/"$/, '')).filter(p => !!p);
                }
                // Handle multi-line parameters if needed... (omitted for brevity in this cleanup step)
            }
            const eventMatch = line.match(/^"([a-zA-Z0-9_]+)"\s*:\s*[\[{]/);
            if (eventMatch && eventMatch[1] !== 'program' && eventMatch[1] !== 'metadata') break;
        }
        return params;
    }
}
