import { createContext, useContext } from "react";
import type { ResourceType } from "../../../hooks/useDataStore";

export type InspectorTab = "fxlang" | "effects" | "special";

export interface FxLangModalTarget {
  type: ResourceType;
  name: string;
  displayName?: string;
  tab?: InspectorTab;
}

export interface FxLangModalContextValue {
  openFxLangModal: (target: FxLangModalTarget) => void;
  closeFxLangModal: () => void;
}

const defaultContextValue: FxLangModalContextValue = {
  openFxLangModal: () => {},
  closeFxLangModal: () => {},
};

export const FxLangModalContext = createContext<FxLangModalContextValue>(defaultContextValue);

export function useFxLangModal(): FxLangModalContextValue {
  return useContext(FxLangModalContext);
}
