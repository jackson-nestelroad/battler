import { useEffect, useMemo, useState } from "react";
import { createPortal } from "react-dom";
import type { ConditionData, ItemData, MoveData, SpeciesData } from "battler-types";
import { type ResourceType, useGenericResource } from "../../../hooks/useDataStore";
import { formatSpeciesClass } from "../../../utils/dataTooltipFormatting";
import cardStyles from "../Tooltip/DataTooltipCard.module.scss";
import {
  extractFxlangData,
  extractItemSpecial,
  extractMoveEffects,
  formatCompactJson,
  linkifyDelegates,
  resolveDelegateTarget,
} from "./fxlangFormatter";
import { highlightFxlangJson } from "./fxlangHighlighter";
import styles from "./FxLangModal.module.scss";

import type { FxLangModalTarget } from "./FxLangModalContext";

export interface FxLangModalProps {
  target: FxLangModalTarget;
  onClose: () => void;
}

type InspectorTab = "fxlang" | "effects" | "special";

const DEFAULT_RESOURCE_SEARCH_ORDER: readonly ResourceType[] = [
  "condition",
  "move",
  "ability",
  "item",
  "species",
];

export default function FxLangModal({ target, onClose }: FxLangModalProps) {
  const [currentTarget, setCurrentTarget] = useState<FxLangModalTarget>(target);
  const [history, setHistory] = useState<FxLangModalTarget[]>([]);

  // Sync state if target prop changes
  useEffect(() => {
    setCurrentTarget(target);
    setHistory([]);
  }, [target]);

  const priority = useMemo<readonly ResourceType[]>(() => {
    if (currentTarget.type === "condition") {
      return DEFAULT_RESOURCE_SEARCH_ORDER;
    }
    return [
      currentTarget.type,
      ...DEFAULT_RESOURCE_SEARCH_ORDER.filter((t) => t !== currentTarget.type),
    ];
  }, [currentTarget.type]);

  const { data: resourceData, loading } = useGenericResource(currentTarget.name, {
    priority,
    include_fxlang: true,
  });

  const resolvedType = resourceData?.type || currentTarget.type;
  const isMove = resolvedType === "move";
  const isItem = resolvedType === "item";
  const [activeTab, setActiveTab] = useState<InspectorTab>("fxlang");
  const [highlightedHtml, setHighlightedHtml] = useState<string>("");

  const displayName =
    (resourceData?.data &&
      "name" in resourceData.data &&
      typeof resourceData.data.name === "string" &&
      resourceData.data.name) ||
    currentTarget.displayName ||
    currentTarget.name;

  const handleNavigateToDelegate = (type: ResourceType, name: string) => {
    setHistory((prev) => [...prev, { ...currentTarget, type: resolvedType, displayName, tab: activeTab }]);
    setCurrentTarget({ type, name });
    setActiveTab("fxlang");
  };

  const handleBack = () => {
    if (history.length === 0) return;
    const previous = history[history.length - 1];
    setHistory((prev) => prev.slice(0, -1));
    setCurrentTarget(previous);
    setActiveTab(previous.tab || "fxlang");
  };

  // Dismiss on Escape key (capture phase so background tooltips don't catch it)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [onClose]);

  // Extract structured effects or fxlang AST
  const {
    fxlangCode,
    effectsCode,
    specialDataCode,
    hasFxlang,
    hasEffects,
    hasSpecialData,
  } = useMemo(() => {
    if (!resourceData?.data) {
      return {
        fxlangCode: "",
        effectsCode: "",
        specialDataCode: "",
        hasFxlang: false,
        hasEffects: false,
        hasSpecialData: false,
      };
    }

    const cleanedFx = extractFxlangData(resourceData.data);
    const cleanedEff = isMove ? extractMoveEffects(resourceData.data as MoveData) : undefined;
    const cleanedSpec = isItem ? extractItemSpecial(resourceData.data as ItemData) : undefined;

    const hasFx = Boolean(cleanedFx && Object.keys(cleanedFx).length > 0);
    const hasEff = Boolean(cleanedEff && Object.keys(cleanedEff).length > 0);
    const hasSpec = Boolean(cleanedSpec && Object.keys(cleanedSpec).length > 0);

    return {
      fxlangCode: hasFx ? formatCompactJson(cleanedFx) : "",
      effectsCode: hasEff ? formatCompactJson(cleanedEff) : "",
      specialDataCode: hasSpec ? formatCompactJson(cleanedSpec) : "",
      hasFxlang: hasFx,
      hasEffects: hasEff,
      hasSpecialData: hasSpec,
    };
  }, [resourceData, isMove, isItem]);

  // Set smart default tab once loaded
  useEffect(() => {
    if (!loading) {
      if (isMove) {
        if (currentTarget.tab === "effects" && hasEffects) {
          setActiveTab("effects");
          return;
        }
        if (currentTarget.tab === "fxlang" && hasFxlang) {
          setActiveTab("fxlang");
          return;
        }
        if (!hasFxlang && hasEffects) {
          setActiveTab("effects");
        } else {
          setActiveTab("fxlang");
        }
      } else if (isItem) {
        if (currentTarget.tab === "special" && hasSpecialData) {
          setActiveTab("special");
          return;
        }
        if (currentTarget.tab === "fxlang" && hasFxlang) {
          setActiveTab("fxlang");
          return;
        }
        if (!hasFxlang && hasSpecialData) {
          setActiveTab("special");
        } else {
          setActiveTab("fxlang");
        }
      } else if (currentTarget.tab) {
        setActiveTab(currentTarget.tab);
      }
    }
  }, [isMove, isItem, loading, hasFxlang, hasEffects, hasSpecialData, currentTarget.tab]);

  const activeCode =
    isMove && activeTab === "effects"
      ? effectsCode
      : isItem && activeTab === "special"
        ? specialDataCode
        : fxlangCode;

  // Run syntax highlighting whenever active code changes
  useEffect(() => {
    let active = true;
    if (!activeCode) {
      setHighlightedHtml("");
      return;
    }

    highlightFxlangJson(activeCode)
      .then((html) => {
        if (active) {
          setHighlightedHtml(linkifyDelegates(html));
        }
      })
      .catch(() => {
        if (active) {
          setHighlightedHtml("");
        }
      });

    return () => {
      active = false;
    };
  }, [activeCode]);

  const navigateDelegateFromElement = (target: EventTarget | null) => {
    const el = (target as HTMLElement | null)?.closest("[data-delegate-prefix]");
    if (!el) return false;
    const prefix = el.getAttribute("data-delegate-prefix");
    const name = el.getAttribute("data-delegate-name");
    if (prefix && name) {
      const resolved = resolveDelegateTarget(prefix, name);
      handleNavigateToDelegate(resolved.type, resolved.name);
      return true;
    }
    return false;
  };

  const handleCodeClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (navigateDelegateFromElement(e.target)) {
      e.preventDefault();
      e.stopPropagation();
    }
  };

  const handleCodeKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "Enter" || e.key === " ") {
      if (navigateDelegateFromElement(e.target)) {
        e.preventDefault();
        e.stopPropagation();
      }
    }
  };

  const conditionData =
    resolvedType === "condition" && resourceData?.data
      ? (resourceData.data as ConditionData)
      : null;
  const speciesData =
    resolvedType === "species" && resourceData?.data
      ? (resourceData.data as SpeciesData)
      : null;

  const subtitle =
    resolvedType === "condition" && conditionData?.condition_type
      ? conditionData.condition_type
      : resolvedType === "species" && speciesData?.class
        ? formatSpeciesClass(speciesData.class)
        : resolvedType.charAt(0).toUpperCase() + resolvedType.slice(1);

  if (typeof document === "undefined") return null;

  return createPortal(
    <div
      className={styles.backdrop}
      onClick={onClose}
      onPointerDown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-labelledby="fxlang-modal-title"
    >
      <div
        className={styles.modal}
        onClick={(e) => e.stopPropagation()}
        onPointerDown={(e) => e.stopPropagation()}
      >
        <header className={styles.modalHeader}>
          <div className="flex-row align-center gap-s">
            {history.length > 0 && (
              <button
                type="button"
                className={styles.backBtn}
                onClick={handleBack}
                aria-label={`Back to ${history[history.length - 1].displayName || history[history.length - 1].name}`}
                title={`Back to ${history[history.length - 1].displayName || history[history.length - 1].name}`}
              >
                ←
              </button>
            )}
            <h2 id="fxlang-modal-title" className={styles.title}>
              {displayName}
            </h2>
            <span className={cardStyles.subtitle}>{subtitle}</span>
          </div>

          {isMove && (
            <div className={cardStyles.tabBar} role="tablist" aria-label="Move effect views">
              <button
                type="button"
                role="tab"
                aria-selected={hasFxlang && activeTab === "fxlang"}
                disabled={!hasFxlang}
                title={hasFxlang ? undefined : "No fxlang callbacks defined"}
                className={cardStyles.tabBtn}
                onClick={() => setActiveTab("fxlang")}
              >
                fxlang
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={hasEffects && activeTab === "effects"}
                disabled={!hasEffects}
                title={hasEffects ? undefined : "No structured effects defined"}
                className={cardStyles.tabBtn}
                onClick={() => setActiveTab("effects")}
              >
                effects
              </button>
            </div>
          )}

          {isItem && (
            <div className={cardStyles.tabBar} role="tablist" aria-label="Item effect views">
              <button
                type="button"
                role="tab"
                aria-selected={hasFxlang && activeTab === "fxlang"}
                disabled={!hasFxlang}
                title={hasFxlang ? undefined : "No fxlang callbacks defined"}
                className={cardStyles.tabBtn}
                onClick={() => setActiveTab("fxlang")}
              >
                fxlang
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={hasSpecialData && activeTab === "special"}
                disabled={!hasSpecialData}
                title={hasSpecialData ? undefined : "No special item data"}
                className={cardStyles.tabBtn}
                onClick={() => setActiveTab("special")}
              >
                special
              </button>
            </div>
          )}

          <button
            type="button"
            aria-label="Close"
            className={styles.closeBtn}
            onClick={onClose}
          >
            ✕
          </button>
        </header>

        <div className={styles.content}>
          {loading ? (
            <div className="flex-col align-center justify-center gap-s py-xl">
              <div className="spinner" />
            </div>
          ) : !activeCode ? (
            <div className={styles.emptyState}>None</div>
          ) : (
            <div
              className={styles.codeContainer}
              onClick={handleCodeClick}
              onKeyDown={handleCodeKeyDown}
              dangerouslySetInnerHTML={{
                __html:
                  highlightedHtml ||
                  `<pre class="shiki"><code><span class="line">${activeCode}</span></code></pre>`,
              }}
            />
          )}
        </div>
      </div>
    </div>,
    document.body,
  );
}
