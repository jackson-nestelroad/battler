import * as vscode from 'vscode';
import { Metadata, FxLangParseResult, FxLangBlock, FxLangLineMapping } from './types';

export class FxLangParser {
    private static eventModifiers = ['ally', 'any', 'field', 'foe', 'side', 'source'];
    private static resolveCache = new Map<string, string | undefined>();
    private static eventNamesSet = new Set<string>();

    constructor(private metadata: Metadata) {
        if (FxLangParser.eventNamesSet.size === 0) {
            this.updateEventNamesSet();
        }
    }

    public static clearResolutionCache() {
        this.resolveCache.clear();
        this.eventNamesSet.clear();
    }

    private updateEventNamesSet() {
        FxLangParser.eventNamesSet = new Set(Object.keys(this.metadata.events || {}));
    }

    public parse(document: vscode.TextDocument, isRelevant: boolean): FxLangParseResult {
        if (!isRelevant) {
            return { blocks: [], mappings: [] };
        }

        const blocks: FxLangBlock[] = [];
        const mappings: FxLangLineMapping[] = [];
        
        let insideFxLangArray = false;
        let fxLangBracketDepth = 0;
        let currentLineIndex = 1;
        let currentBlockStart = -1;

        for (let i = 0; i < document.lineCount; i++) {
            const line = document.lineAt(i).text;
            const trimmed = line.trim();

            if (!insideFxLangArray) {
                if (!trimmed.startsWith('"') || !trimmed.includes('[')) continue;

                const match = trimmed.match(/^"([a-zA-Z0-9_]+)"\s*:\s*\[/);
                if (match) {
                    const rawName = match[1];
                    const resolved = this.resolveEventName(rawName);
                    if (resolved || rawName === 'program') {
                        insideFxLangArray = true;
                        fxLangBracketDepth = 1;
                        currentLineIndex = 1;
                        currentBlockStart = i;
                        let eventName = resolved;

                        if (rawName === 'program') {
                            eventName = this.findInheritedEvent(document, i);
                            if (eventName) {
                                // Find where the event block actually starts
                                for (let j = i - 1; j >= 0; j--) {
                                    const prevLine = document.lineAt(j).text.trim();
                                    const prevMatch = prevLine.match(/^"([a-zA-Z0-9_]+)"\s*:\s*[\[\{]/);
                                    if (prevMatch && this.resolveEventName(prevMatch[1]) === eventName) {
                                        currentBlockStart = j;
                                        break;
                                    }
                                }
                            }
                        }

                        if (trimmed.endsWith(']') || trimmed.endsWith('],')) {
                            this.processInlineMappings(line, i, mappings);
                            blocks.push({ startLine: currentBlockStart, endLine: i, eventName });
                            insideFxLangArray = false;
                        }
                    }
                }
            } else {
                const open = (trimmed.match(/\[/g) || []).length;
                const close = (trimmed.match(/\]/g) || []).length;
                fxLangBracketDepth += open - close;

                if (fxLangBracketDepth <= 0) {
                    const block = blocks[blocks.length - 1];
                    blocks.push({ startLine: currentBlockStart, endLine: i, eventName: block?.eventName });
                    insideFxLangArray = false;
                    continue;
                }

                this.processLineMappings(line, i, currentLineIndex, mappings);
                currentLineIndex += (line.match(/"[^"]*"/g) || []).length;
            }
        }
        return { blocks, mappings };
    }

    private findInheritedEvent(document: vscode.TextDocument, lineIdx: number): string | undefined {
        for (let j = lineIdx - 1; j >= 0; j--) {
            const prevLine = document.lineAt(j).text.trim();
            const prevMatch = prevLine.match(/^"([a-zA-Z0-9_]+)"\s*:\s*[\[\{]/);
            if (prevMatch) {
                const prevRaw = prevMatch[1];
                const prevResolved = this.resolveEventName(prevRaw);
                if (prevResolved && prevRaw !== 'program' && prevRaw !== 'metadata') {
                    return prevResolved;
                }
            }
        }
        return undefined;
    }

    private processInlineMappings(line: string, lineIdx: number, mappings: FxLangLineMapping[]) {
        const stringMatches = line.match(/"([^"]*)"/g);
        if (stringMatches && stringMatches.length > 1) {
            let lastIdx = 0;
            for (let s = 1; s < stringMatches.length; s++) {
                const matchedStr = stringMatches[s];
                const str = matchedStr.substring(1, matchedStr.length - 1);
                const strIdx = line.indexOf('"' + str + '"', lastIdx);
                if (strIdx !== -1) {
                    lastIdx = strIdx + str.length + 2;
                    mappings.push({
                        documentLine: lineIdx,
                        charStart: strIdx,
                        charEnd: strIdx + str.length + 2,
                        lineIndex: s
                    });
                }
            }
        }
    }

    private processLineMappings(line: string, lineIdx: number, startIndex: number, mappings: FxLangLineMapping[]) {
        const stringMatches = line.match(/"([^"]*)"/g);
        if (stringMatches) {
            let lastIdx = 0;
            for (let s = 0; s < stringMatches.length; s++) {
                const matchedStr = stringMatches[s];
                const str = matchedStr.substring(1, matchedStr.length - 1);
                const strIdx = line.indexOf('"' + str + '"', lastIdx);
                if (strIdx !== -1) {
                    lastIdx = strIdx + str.length + 2;
                    mappings.push({
                        documentLine: lineIdx,
                        charStart: strIdx,
                        charEnd: strIdx + str.length + 2,
                        lineIndex: startIndex + s
                    });
                }
            }
        }
    }

    public resolveEventName(rawName: string): string | undefined {
        if (FxLangParser.resolveCache.has(rawName)) return FxLangParser.resolveCache.get(rawName);

        if (FxLangParser.eventNamesSet.size === 0) this.updateEventNamesSet();
        
        let result: string | undefined = undefined;
        if (FxLangParser.eventNamesSet.has(rawName)) {
            result = rawName;
        } else if (rawName.startsWith('on_')) {
            let base = rawName.substring(3);
            if (FxLangParser.eventNamesSet.has(base)) {
                result = base;
            } else {
                for (const mod of FxLangParser.eventModifiers) {
                    if (base.startsWith(mod + '_')) {
                        let deeperBase = base.substring(mod.length + 1);
                        if (FxLangParser.eventNamesSet.has(deeperBase)) {
                            result = deeperBase;
                            break;
                        }
                    }
                }
            }
        }
        
        FxLangParser.resolveCache.set(rawName, result);
        return result;
    }
}
