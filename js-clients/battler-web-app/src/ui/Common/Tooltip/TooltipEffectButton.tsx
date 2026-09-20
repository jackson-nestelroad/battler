import type { ResourceType } from "../../../hooks/useDataStore";
import { type InspectorTab, useFxLangModal } from "../FxLang/FxLangModalContext";
import styles from "./TooltipEffectButton.module.scss";

export interface TooltipEffectButtonProps {
  type: ResourceType;
  name: string;
  displayName?: string;
  tab?: InspectorTab;
}

export default function TooltipEffectButton({
  type,
  name,
  displayName,
  tab,
}: TooltipEffectButtonProps) {
  const { openFxLangModal } = useFxLangModal();
  const label = `View effect details for ${displayName || name}`;

  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={styles.effectBtn}
      onClick={(e) => {
        e.stopPropagation();
        openFxLangModal({ type, name, displayName, tab });
      }}
    >
      Effect
    </button>
  );
}
