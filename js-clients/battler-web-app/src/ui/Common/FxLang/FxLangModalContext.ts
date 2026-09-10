import { createContext, useContext } from "react";
import type { ResourceType } from "../../../hooks/useDataStore";

export interface FxLangModalTarget {
  type: ResourceType;
  name: string;
  displayName?: string;
  tab?: "fxlang" | "effects" | "special";
}

export interface FxLangModalContextValue {
  openFxLangModal: (target: FxLangModalTarget) => void;
  closeFxLangModal: () => void;
}

export const FxLangModalContext = createContext<FxLangModalContextValue | null>(null);

export function useFxLangModal(): FxLangModalContextValue {
  const context = useContext(FxLangModalContext);
  if (!context) {
    throw new Error("useFxLangModal must be used within a FxLangModalProvider");
  }
  return context;
}
