import type { ResourceType } from "../../../hooks/useDataStore";
import { useFxLangModal } from "../FxLang/FxLangModalContext";
import cardStyles from "./DataTooltipCard.module.scss";

export interface TooltipEffectButtonProps {
  type: ResourceType;
  name: string;
}

export default function TooltipEffectButton({
  type,
  name,
}: TooltipEffectButtonProps) {
  const { openFxLangModal } = useFxLangModal();

  return (
    <button
      type="button"
      className={cardStyles.effectBtn}
      onClick={(e) => {
        e.stopPropagation();
        openFxLangModal({ type, name });
      }}
    >
      Effect
    </button>
  );
}
