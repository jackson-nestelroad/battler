import { useEffect } from "react";
import type {
  AbilityData,
  ItemData,
  MoveData,
  ResourceType,
  SpeciesData,
} from "battler-data-service-client";
import { useResourceData } from "../../../hooks/useDataStore";
import AbilityTooltipCard from "../../Common/Tooltip/AbilityTooltipCard";
import ItemTooltipCard from "../../Common/Tooltip/ItemTooltipCard";
import MoveTooltipCard from "../../Common/Tooltip/MoveTooltipCard";
import SpeciesTooltipCard from "../../Common/Tooltip/SpeciesTooltipCard";
import type { DexTab } from "./DexCatalogItem";
import styles from "./DexInspector.module.scss";

export interface DexInspectorProps {
  tab: DexTab;
  selectedName: string | null;
  onClose: () => void;
}

const TAB_TO_RESOURCE: Record<DexTab, ResourceType> = {
  species: "species",
  moves: "move",
  abilities: "ability",
  items: "item",
};

export default function DexInspector({
  tab,
  selectedName,
  onClose,
}: DexInspectorProps) {
  const resourceType = TAB_TO_RESOURCE[tab];
  const { data, description, loading } = useResourceData(
    resourceType,
    selectedName,
  );

  // Close drawer on Escape key when on mobile
  useEffect(() => {
    if (!selectedName) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedName, onClose]);

  const closeButton = (
    <button
      type="button"
      className={styles.closeButton}
      onClick={onClose}
      title="Close details"
      aria-label="Close details"
    >
      ✕
    </button>
  );

  const renderCardBody = () => {
    if (loading) {
      return (
        <div className={styles.loadingContainer} role="status">
          <span className="spinner spinner-sm" />
          <span>Loading...</span>
        </div>
      );
    }

    if (!data) {
      return (
        <div className={styles.emptyPlaceholder} role="status">
          None
        </div>
      );
    }

    switch (resourceType) {
      case "species":
        return (
          <SpeciesTooltipCard
            data={data as SpeciesData}
            description={description}
            headerAction={closeButton}
          />
        );
      case "move":
        return (
          <MoveTooltipCard
            data={data as MoveData}
            description={description}
            headerAction={closeButton}
          />
        );
      case "ability":
        return (
          <AbilityTooltipCard
            data={data as AbilityData}
            description={description}
            headerAction={closeButton}
          />
        );
      case "item":
        return (
          <ItemTooltipCard
            data={data as ItemData}
            description={description}
            headerAction={closeButton}
          />
        );
      default:
        return null;
    }
  };

  const isCardReady = selectedName && !loading && Boolean(data);

  const renderInspectorContent = () => {
    if (isCardReady) {
      return <div className={styles.cardContent}>{renderCardBody()}</div>;
    }
    return (
      <>
        <header className={styles.fallbackHeader}>
          <span className={styles.inspectorTitle}>{selectedName}</span>
          {closeButton}
        </header>
        <div className={styles.cardContent}>{renderCardBody()}</div>
      </>
    );
  };

  return (
    <>
      {/* Desktop Sticky Card */}
      <aside className={styles.desktopInspector} aria-label="Resource Inspector">
        {!selectedName ? (
          <div className={styles.emptyPlaceholder}>None</div>
        ) : (
          renderInspectorContent()
        )}
      </aside>

      {/* Mobile Centered Modal */}
      {selectedName && (
        <>
          <div
            className={styles.modalBackdrop}
            onClick={onClose}
            aria-hidden="true"
          />
          <div
            className={styles.modalSheet}
            role="dialog"
            aria-modal="true"
            aria-label={`${selectedName} details`}
          >
            {renderInspectorContent()}
          </div>
        </>
      )}
    </>
  );
}
