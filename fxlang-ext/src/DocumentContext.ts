import * as vscode from 'vscode';
import { Metadata, FxLangParseResult, SymbolTable } from './types';
import { FxLangParser } from './Parser';

export interface CachedContext {
    version: number;
    parseResult: FxLangParseResult;
    symbolTable?: SymbolTable;
    blockTypeCache: Map<string, 'array' | 'object' | 'none'>;
}

export class DocumentContextManager {
    private cache = new Map<string, CachedContext>();
    private relevanceCache = new Map<string, { version: number, relevant: boolean }>();

    constructor(private metadata: Metadata) {}

    public updateMetadata(metadata: Metadata) {
        this.metadata = metadata;
        this.cache.clear();
        // Relevance cache can stay as it doesn't depend on metadata content, 
        // just on the presence of "callbacks"/"program" strings.
    }

    public getContext(document: vscode.TextDocument): FxLangParseResult {
        const uri = document.uri.toString();
        const cached = this.cache.get(uri);

        if (cached && cached.version === document.version) {
            return cached.parseResult;
        }

        const parser = new FxLangParser(this.metadata);
        const result = parser.parse(document, this.isRelevant(document));
        
        this.cache.set(uri, {
            version: document.version,
            parseResult: result,
            blockTypeCache: new Map()
        });

        return result;
    }

    public isRelevant(document: vscode.TextDocument): boolean {
        if (document.languageId === 'fxlang') return true;
        if (document.languageId !== 'json' && document.languageId !== 'jsonc') return false;

        const uri = document.uri.toString();
        const cached = this.relevanceCache.get(uri);
        if (cached && cached.version === document.version) return cached.relevant;

        let relevant = false;
        for (let i = 0; i < Math.min(document.lineCount, 5000); i++) {
            const line = document.lineAt(i).text;
            if (line.includes('"callbacks"') || line.includes('"program"')) {
                relevant = true;
                break;
            }
        }

        this.relevanceCache.set(uri, { version: document.version, relevant });
        return relevant;
    }

    public getEnclosingBlockType(document: vscode.TextDocument, position: vscode.Position): 'array' | 'object' | 'none' {
        const uri = document.uri.toString();
        const context = this.cache.get(uri);
        if (!context || context.version !== document.version) {
            this.getContext(document);
        }
        
        const key = `${position.line}:${position.character}`;
        const cached = this.cache.get(uri)?.blockTypeCache.get(key);
        if (cached) return cached;

        const parseResult = this.getContext(document);
        
        // Fast path
        if (parseResult.blocks.some(b => position.line >= b.startLine && position.line <= b.endLine)) {
            this.cache.get(uri)?.blockTypeCache.set(key, 'array');
            return 'array';
        }

        // Slow path
        const text = document.getText(new vscode.Range(new vscode.Position(0, 0), position));
        const stack: ('array' | 'object')[] = [];
        let inString = false;
        let escape = false;

        for (let i = 0; i < text.length; i++) {
            const char = text[i];
            if (escape) { escape = false; continue; }
            if (char === '\\') { escape = true; continue; }
            if (char === '"') { inString = !inString; continue; }
            if (!inString) {
                if (char === '[') stack.push('array');
                else if (char === ']') stack.pop();
                else if (char === '{') stack.push('object');
                else if (char === '}') stack.pop();
            }
        }
        
        const result = stack.length > 0 ? stack[stack.length - 1] : 'none';
        this.cache.get(uri)?.blockTypeCache.set(key, result);
        return result;
    }

    public clear(uri: string) {
        this.cache.delete(uri);
        this.relevanceCache.delete(uri);
    }
}
