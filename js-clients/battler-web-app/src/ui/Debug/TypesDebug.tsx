import { useState } from "react";
import { setCurrentView } from "../../store/battlesSlice";
import { useAppDispatch } from "../../store/store";
import TypeBadge from "../Common/TypeBadge";
import styles from "./TypesDebug.module.scss";

const ALL_TYPES = [
  "Normal",
  "Fighting",
  "Flying",
  "Poison",
  "Ground",
  "Rock",
  "Bug",
  "Ghost",
  "Steel",
  "Fire",
  "Water",
  "Grass",
  "Electric",
  "Psychic",
  "Ice",
  "Dragon",
  "Dark",
  "Fairy",
  "Stellar",
  "???",
];

export default function TypesDebug() {
  const dispatch = useAppDispatch();
  const [size, setSize] = useState<"sm" | "md">("md");
  const [showIcon, setShowIcon] = useState(true);
  const [fixedWidth, setFixedWidth] = useState(false);

  return (
    <div className={styles.container}>
      <header className={styles.headerRow}>
        <div className="flex-row align-center gap-s">
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            onClick={() => dispatch(setCurrentView("lobby"))}
          >
            ← Lobby
          </button>
          <h1 className={styles.title}>Type Badges Debug</h1>
        </div>
      </header>

      {/* Interactive Controls */}
      <section className={`card ${styles.controlsCard}`}>
        <div className={styles.controlsRow}>
          <div className={styles.controlGroup}>
            <span className={styles.controlLabel}>Size:</span>
            <button
              type="button"
              className={`btn btn-sm ${size === "md" ? "btn-primary" : "btn-secondary"}`}
              onClick={() => setSize("md")}
            >
              Medium
            </button>
            <button
              type="button"
              className={`btn btn-sm ${size === "sm" ? "btn-primary" : "btn-secondary"}`}
              onClick={() => setSize("sm")}
            >
              Small
            </button>
          </div>

          <div className={styles.controlGroup}>
            <span className={styles.controlLabel}>Icon:</span>
            <button
              type="button"
              className={`btn btn-sm ${showIcon ? "btn-primary" : "btn-secondary"}`}
              onClick={() => setShowIcon((v) => !v)}
            >
              {showIcon ? "With Icon" : "Text Only"}
            </button>
          </div>

          <div className={styles.controlGroup}>
            <span className={styles.controlLabel}>Uniform Width:</span>
            <button
              type="button"
              className={`btn btn-sm ${fixedWidth ? "btn-primary" : "btn-secondary"}`}
              onClick={() => setFixedWidth((v) => !v)}
            >
              {fixedWidth ? "Fixed Width" : "Auto Width"}
            </button>
          </div>
        </div>
      </section>

      {/* All Types Grid */}
      <section className="flex-col gap-m">
        <h2 className={styles.comparisonCardTitle}>All Types ({ALL_TYPES.length})</h2>
        <div className={styles.typesGrid}>
          {ALL_TYPES.map((type) => {
            const normalized = type.toLowerCase() === "???" ? "unknown" : type.toLowerCase();
            return (
              <div key={type} className={styles.typeCard}>
                <TypeBadge
                  type={type}
                  size={size}
                  showIcon={showIcon}
                  fixedWidth={fixedWidth}
                />
                <div className={styles.typeMeta}>
                  <span className={styles.typeName}>{type}</span>
                  <span className={styles.typeVar}>--color-type-{normalized}</span>
                </div>
              </div>
            );
          })}
        </div>
      </section>

      {/* Consistent Width Comparison */}
      <section className={styles.comparisonSection}>
        <h2 className={styles.comparisonCardTitle}>Width Consistency: Auto vs Fixed</h2>
        <div className={styles.comparisonGrid}>
          <div className={styles.comparisonCard}>
            <h3 className={styles.comparisonCardTitle}>Auto-Fit Width</h3>
            <p className={styles.comparisonCardDesc}>
              Width varies based on character count (e.g. &quot;Bug&quot; vs &quot;Electric&quot;).
            </p>
            <div className={styles.badgeList}>
              {["Bug", "Ice", "Fire", "Fighting", "Electric"].map((t) => (
                <div key={t} className={styles.badgeRow}>
                  <TypeBadge type={t} size="md" fixedWidth={false} />
                  <span className={styles.badgeLabel}>{t}</span>
                </div>
              ))}
            </div>
          </div>

          <div className={styles.comparisonCard}>
            <h3 className={styles.comparisonCardTitle}>Fixed Uniform Width</h3>
            <p className={styles.comparisonCardDesc}>
              Consistent width (var(--type-badge-width-md)) across all types with centered content.
            </p>
            <div className={styles.badgeList}>
              {["Bug", "Ice", "Fire", "Fighting", "Electric"].map((t) => (
                <div key={t} className={styles.badgeRow}>
                  <TypeBadge type={t} size="md" fixedWidth={true} />
                  <span className={styles.badgeLabel}>{t}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* Border Radius Comparison */}
      <section className={styles.comparisonSection}>
        <h2 className={styles.comparisonCardTitle}>
          Border Radius: 50% (--border-radius-round) vs 9999px (--border-radius-pill)
        </h2>
        <div className={styles.comparisonGrid}>
          <div className={styles.comparisonCard}>
            <h3 className={styles.comparisonCardTitle}>50% (--border-radius-round)</h3>
            <p className={styles.comparisonCardDesc}>
              On a non-square rectangle (e.g. 92px × 20px), 50% calculates a 46px horizontal radius
              and a 10px vertical radius, curving the top/bottom edges into an <strong>ellipse/egg</strong> shape.
            </p>
            <div className="flex-row align-center gap-s">
              <span className={styles.demoEllipseBadge}>
                <img
                  src={`${import.meta.env?.BASE_URL ?? "/"}assets/types/electric.png`}
                  alt=""
                  width={14}
                  height={14}
                />
                <span>Electric</span>
              </span>
              <span className={styles.badgeLabel}>(Oval / Ellipse)</span>
            </div>
          </div>

          <div className={styles.comparisonCard}>
            <h3 className={styles.comparisonCardTitle}>9999px (--border-radius-pill)</h3>
            <p className={styles.comparisonCardDesc}>
              Clamps each corner radius to half the height (10px), keeping the top and bottom
              borders completely <strong>straight and flat</strong> with semicircular ends.
            </p>
            <div className="flex-row align-center gap-s">
              <span className={styles.demoPillBadge}>
                <img
                  src={`${import.meta.env?.BASE_URL ?? "/"}assets/types/electric.png`}
                  alt=""
                  width={14}
                  height={14}
                />
                <span>Electric</span>
              </span>
              <span className={styles.badgeLabel}>(Capsule Pill)</span>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
