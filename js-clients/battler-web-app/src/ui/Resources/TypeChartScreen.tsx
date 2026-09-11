import TypeChartGrid from "../Common/TypeChartGrid";
import styles from "./TypeChartScreen.module.scss";

export interface TypeChartScreenProps {
  onBack: () => void;
}

export default function TypeChartScreen({ onBack }: TypeChartScreenProps) {
  return (
    <div className="page-container">
      <header className="screen-header flex-row justify-between align-center gap-m">
        <div className="flex-row align-center gap-m">
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            onClick={onBack}
            title="Back to Resources"
          >
            ← Resources
          </button>
          <div className="screen-header-title flex-col gap-xs">
            <h2>Type Chart</h2>
          </div>
        </div>
      </header>

      <div className={styles.gridContainer}>
        <TypeChartGrid />
      </div>
    </div>
  );
}

