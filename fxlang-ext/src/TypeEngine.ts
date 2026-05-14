import { Metadata, SymbolTable, MemberData } from './types';

export class TypeEngine {
    private typeMembersCache: Record<string, Record<string, MemberData>> = {};
    private commonFlagsSet = new Set<string>();
    private typeSplitCache = new Map<string, Set<string>>();

    constructor(public metadata: Metadata) {
        this.preprocess();
    }

    private preprocess() {
        this.commonFlagsSet = new Set(this.metadata.common_flags || []);
        
        this.typeMembersCache = {};
        for (const type of Object.keys(this.metadata.type_members)) {
            this.typeMembersCache[type] = this.internalGetTypeMembers(type);
        }
        this.typeMembersCache['ActiveMove'] = this.internalGetTypeMembers('ActiveMove');
    }

    public updateMetadata(metadata: Metadata) {
        this.metadata = metadata;
        this.typeSplitCache.clear();
        this.preprocess();
    }

    private internalGetTypeMembers(type: string): Record<string, MemberData> {
        const members: Record<string, MemberData> = {};
        const baseType = type.includes(' | ') ? type.split(' | ')[0] : type;
        
        if (baseType === 'ActiveMove') {
            const effectMembers = this.metadata.type_members['Effect'];
            if (effectMembers) Object.assign(members, effectMembers);
        }
        
        const specificMembers = this.metadata.type_members[baseType];
        if (specificMembers) Object.assign(members, specificMembers);
        
        return members;
    }

    public getTypeMembers(type: string): Record<string, MemberData> {
        const baseType = type.includes(' | ') ? type.split(' | ')[0] : type;
        return this.typeMembersCache[baseType] || this.metadata.type_members[baseType] || {};
    }

    public inferType(expression: string, symbols: SymbolTable, eventName?: string): string | undefined {
        expression = expression.trim();
        
        if (expression === 'undefined') return 'Undefined';
        if (expression.match(/^(true|false)$/)) return 'Boolean';
        if (expression.match(/^[+-]\d+(\.\d+)?$/)) return 'Fraction';
        if (expression.match(/^\d+(\.\d+)?$/)) return 'UFraction';
        if (expression.match(/^[+-]?\d+(?:\.\d+)?(?:\s*[\+\-\*\/]\s*\d+(?:\.\d+)?)+$/)) {
            return expression.startsWith('-') ? 'Fraction' : 'UFraction';
        }
        if (expression.match(/^['"]/)) return 'String';
        
        if (expression.startsWith('[') && expression.endsWith(']')) {
            return this.inferListType(expression, symbols, eventName);
        }

        const funcMatch = expression.match(/^([a-z0-9_]+)\(/);
        if (funcMatch) {
            return this.inferFunctionReturnType(expression, funcMatch[1], symbols, eventName);
        }
        
        const chainMatch = expression.match(/^(\$[a-zA-Z0-9_]+(?:\.[a-zA-Z0-9_]+)*)/);
        if (chainMatch) {
            return this.resolveChainType(chainMatch[1].split('.'), symbols, eventName);
        }
        
        if (expression.match(/^[a-zA-Z0-9_]+$/)) return 'String';
        return undefined;
    }

    private inferListType(expression: string, symbols: SymbolTable, eventName?: string): string {
        const innerText = expression.substring(1, expression.length - 1).trim();
        if (!innerText) return 'List';
        
        const elements = innerText.split(',').map(e => e.trim());
        let commonType: string | undefined = undefined;
        let uniform = true;
        
        const checkLimit = Math.min(elements.length, 5);
        for (let i = 0; i < checkLimit; i++) {
            const elType = this.inferType(elements[i], symbols, eventName);
            if (!elType || elType === 'unknown') {
                uniform = false;
                break;
            }
            if (commonType === undefined) commonType = elType;
            else if (commonType !== elType) {
                uniform = false;
                break;
            }
        }
        
        return (uniform && commonType) ? `List<${commonType}>` : 'List';
    }

    private inferFunctionReturnType(expression: string, funcName: string, symbols: SymbolTable, eventName?: string): string | undefined {
        if (funcName === 'func_call') {
            const innerMatch = expression.match(/^func_call\(\s*([a-z0-9_]+)/);
            if (innerMatch) funcName = innerMatch[1];
        }
        
        const funcMeta = this.metadata.functions[funcName];
        if (!funcMeta) return undefined;

        if (funcMeta.returns_item_from_list) {
            const argsString = this.extractFunctionArguments(expression);
            if (argsString) {
                const firstArg = argsString.split(',')[0].trim();
                const listType = this.inferType(firstArg, symbols, eventName);
                return this.unwrapListType(listType);
            }
            return 'unknown';
        }
        
        return (funcMeta.type === 'List' && funcMeta.item_type) ? `List<${funcMeta.item_type}>` : funcMeta.type;
    }

    public resolveChainType(chain: string[], symbols: SymbolTable, eventName?: string): string | undefined {
        if (chain.length === 0) return undefined;

        let currentType: string | undefined;
        const first = chain[0];

        if (first.startsWith('$')) {
            const varName = first.substring(1);
            currentType = symbols[varName];
            if (!currentType) {
                const varData = this.getVariableData(varName, eventName);
                if (varData) currentType = this.getDisplayType(varData.type, varData.item_type);
            }
        } else {
            const member = this.metadata.variable_members[first];
            if (member) currentType = this.getDisplayType(member.type, member.item_type);
        }

        for (let i = 1; i < chain.length; i++) {
            const memberName = chain[i];
            if (!currentType) {
                if (this.metadata.variable_members[memberName]) {
                    currentType = this.metadata.variable_members[memberName].type;
                    continue;
                }
                return undefined;
            }
            
            const typeMembers = this.getTypeMembers(currentType);
            let member = typeMembers[memberName] || this.metadata.variable_members[memberName];
            if (!member) return undefined;
            
            currentType = this.getDisplayType(member.type, member.item_type);
        }

        return currentType;
    }

    public getVariableData(varName: string, eventName?: string) {
        if (eventName && this.metadata.events?.[eventName]?.variables?.[varName]) {
            const vData = this.metadata.events[eventName].variables[varName];
            return { ...vData, origin: `Event Context: ${eventName}` };
        } else if (this.metadata.variables[varName]) {
            return { ...this.metadata.variables[varName], origin: 'Built-in / Global' };
        }
        return undefined;
    }

    public areTypesCompatible(parentType: string, paramType: string): boolean {
        if (paramType === 'Any') return true;
        const parentTypes = this.getSplitTypes(parentType);
        const paramTypes = this.getSplitTypes(paramType);
        
        for (const pt of parentTypes) {
            if (paramTypes.has(pt)) return true;
        }
        
        for (const pt of parentTypes) {
            if (paramTypes.has('Effect') && pt === 'ActiveMove') return true;
            if (paramTypes.has('Object') && (pt === 'BoostTable' || pt === 'StatTable' || pt === 'EffectState')) return true;
            if (paramTypes.has('Fraction') && pt === 'UFraction') return true;
            if (paramTypes.has('UFraction') && pt === 'Fraction') return true;
        }
        
        return false;
    }

    private getSplitTypes(typeStr: string): Set<string> {
        let cached = this.typeSplitCache.get(typeStr);
        if (cached) return cached;
        const split = new Set(typeStr.split(' | '));
        this.typeSplitCache.set(typeStr, split);
        return split;
    }

    public getDisplayType(typeStr: string, itemType?: string): string {
        return (typeStr.includes('List') && itemType) ? typeStr.replace('List', `List<${itemType}>`) : typeStr;
    }

    public unwrapListType(typeStr: string | undefined): string {
        if (!typeStr) return 'unknown';
        const match = typeStr.match(/List<([^>]+)>/);
        return match ? match[1] : 'unknown';
    }

    private extractFunctionArguments(expression: string): string {
        const match = expression.match(/^([a-z0-9_]+)\((.*)\)$/);
        if (!match) return '';
        if (match[1] === 'func_call') {
            const parts = match[2].split(':');
            return parts.length > 1 ? parts.slice(1).join(':').trim() : '';
        }
        return match[2].trim();
    }
}
