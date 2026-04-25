export interface MemberData {
    description: string;
    type: string;
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
}

export interface VariableData {
    type: string;
    optional: boolean;
}

export interface Metadata {
    variables: Record<string, VariableData>;
    variable_members: Record<string, MemberData>;
    type_members: Record<string, Record<string, MemberData>>;
    functions: Record<string, FunctionData>;
    events: Record<string, { description: string; variables: Record<string, VariableData> }>;
    common_flags: string[];
}

export type SymbolTable = Record<string, string>;
