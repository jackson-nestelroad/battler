import { useCallback, useState, type ReactNode } from "react";
import FxLangModal from "./FxLangModal";
import { FxLangModalContext, type FxLangModalTarget } from "./FxLangModalContext";

export interface FxLangModalProviderProps {
  children: ReactNode;
}

export default function FxLangModalProvider({ children }: FxLangModalProviderProps) {
  const [target, setTarget] = useState<FxLangModalTarget | null>(null);

  const openFxLangModal = useCallback((newTarget: FxLangModalTarget) => {
    setTarget(newTarget);
  }, []);

  const closeFxLangModal = useCallback(() => {
    setTarget(null);
  }, []);

  return (
    <FxLangModalContext.Provider value={{ openFxLangModal, closeFxLangModal }}>
      {children}
      {target && <FxLangModal target={target} onClose={closeFxLangModal} />}
    </FxLangModalContext.Provider>
  );
}
