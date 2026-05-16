import * as vscode from 'vscode';

export interface MemberData {
    description: string;
    type: string;
    item_type?: string;
    only_applicable_to_move?: boolean;
    only_applicable_to_active_move?: boolean;
}

export interface ParameterData {
    name: string;
    type: string;
    description: string;
    optional: boolean;
}

export interface FlagData {
    name: string;
    description: string;
}

export interface FunctionData {
    description: string;
    parameters: ParameterData[];
    flags: FlagData[];
    type: string;
    item_type?: string;
    returns_item_from_list?: boolean;
}

export interface VariableData {
    type: string;
    optional: boolean;
    item_type?: string;
}

export interface Metadata {
    variables: Record<string, VariableData>;
    variable_members: Record<string, MemberData>;
    type_members: Record<string, Record<string, MemberData>>;
    functions: Record<string, FunctionData>;
    events: Record<string, { description: string; variables: Record<string, VariableData>; allows_custom_input_vars?: boolean }>;
    common_flags: string[];
}

export type SymbolTable = Record<string, string>;

export interface FxLangBlock {
    startLine: number;
    endLine: number;
    eventName?: string;
}

export interface FxLangLineMapping {
    documentLine: number;
    charStart: number;
    charEnd: number;
    lineIndex: number;
}

export interface FxLangParseResult {
    blocks: FxLangBlock[];
    mappings: FxLangLineMapping[];
}
