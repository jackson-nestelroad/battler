import TypeChartGrid from "../Common/TypeChartGrid";
import styles from "./TypeChartStandaloneApp.module.scss";

export default function TypeChartStandaloneApp() {
  const baseUrl = import.meta.env.BASE_URL || "/";

  return (
    <div className="page-container scroll-y">
      <header className="screen-header flex-row justify-between align-center gap-m">
        <div className="flex-row align-center gap-m">
          <img
            src={`${baseUrl}favicon.svg`}
            alt="Battler"
            className={styles.logoIcon}
          />
          <div className="screen-header-title flex-col gap-xs">
            <h2>Type Chart</h2>
          </div>
        </div>

        <a
          href={baseUrl}
          className="btn btn-secondary btn-sm"
          title="Back to Battler"
        >
          ← Battler
        </a>
      </header>

      <div className="flex-col align-center w-full">
        <TypeChartGrid />
      </div>
    </div>
  );
}
