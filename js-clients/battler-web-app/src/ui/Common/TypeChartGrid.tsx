import { useEffect, useMemo, useState } from "react";
import {
  ALL_POKEMON_TYPES,
  formatMultiplierValue,
  getDefensiveMultipliers,
  getEffectiveness,
  useTypeChart,
} from "../../hooks/useTypeChart";
import {
  formatEffectivenessComparison,
  getMultiplierClass,
} from "../../utils/typeEffectiveness";
import TypeBadge from "./TypeBadge";
import styles from "./TypeChartGrid.module.scss";

export interface TypeChartGridProps {
  defendingTypes?: string[];
  onDefendersChange?: (defenders: string[]) => void;
  className?: string;
}

export default function TypeChartGrid({
  defendingTypes,
  onDefendersChange,
  className = "",
}: TypeChartGridProps) {
  const { typeChart, loading, error } = useTypeChart();
  const [selectedDefenders, setSelectedDefenders] = useState<string[]>(
    defendingTypes ?? []
  );
  const [hoveredRow, setHoveredRow] = useState<string | null>(null);
  const [hoveredCol, setHoveredCol] = useState<string | null>(null);

  useEffect(() => {
    if (defendingTypes !== undefined) {
      setSelectedDefenders(defendingTypes);
    }
  }, [defendingTypes]);

  const handleToggleDefender = (type: string) => {
    setSelectedDefenders((prev) => {
      let next: string[];
      if (prev.includes(type)) {
        next = prev.filter((t) => t !== type);
      } else {
        next = [...prev, type];
      }
      onDefendersChange?.(next);
      return next;
    });
  };

  const handleClear = () => {
    setSelectedDefenders([]);
    onDefendersChange?.([]);
  };

  const combinedMultipliers = useMemo(() => {
    if (selectedDefenders.length === 0) return null;
    return getDefensiveMultipliers(typeChart, selectedDefenders);
  }, [typeChart, selectedDefenders]);

  const remainingTypes = useMemo(() => {
    return ALL_POKEMON_TYPES.filter((t) => !selectedDefenders.includes(t));
  }, [selectedDefenders]);

  if (loading) {
    return (
      <div className={styles.loadingContainer}>
        <div className="spinner" />
        <p className="text-secondary">Loading...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="alert alert-danger w-full">
        <span className="alert-message">{error}</span>
      </div>
    );
  }

  const hasSelection = selectedDefenders.length > 0;

  return (
    <div className={`${styles.gridContainer} ${className}`.trim()}>
      <div
        className={styles.gridWrapper}
        onMouseLeave={() => {
          setHoveredRow(null);
          setHoveredCol(null);
        }}
      >
        <table className={styles.table}>
          <thead>
            <tr>
              <th
                className={styles.thCorner}
                onMouseEnter={() => {
                  setHoveredRow(null);
                  setHoveredCol(null);
                }}
                title={
                  hasSelection
                    ? "Reset defender selection"
                    : "Rows = Attacking type (ATK ↓) | Columns = Defending type (DEF →)"
                }
              >
                {hasSelection ? (
                  <button
                    type="button"
                    className={styles.cornerResetBtn}
                    onClick={handleClear}
                    title="Reset defender selection"
                    aria-label="Reset defender selection"
                  >
                    <svg
                      className={styles.resetIcon}
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2.5"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      aria-hidden="true"
                    >
                      <line x1="18" y1="6" x2="6" y2="18" />
                      <line x1="6" y1="6" x2="18" y2="18" />
                    </svg>
                  </button>
                ) : (
                  <div className={styles.cornerHeader}>
                    <span
                      className={styles.cornerAxisDef}
                      title="Defending type across columns (DEF →)"
                    >
                      DEF →
                    </span>
                    <span
                      className={styles.cornerAxisAtk}
                      title="Attacking type down rows (ATK ↓)"
                    >
                      ATK ↓
                    </span>
                  </div>
                )}
              </th>

              {/* Selected Defender Columns */}
              {selectedDefenders.map((defType) => {
                const isColHovered = hoveredCol === defType;

                return (
                  <th
                    key={defType}
                    className={`${styles.thDefender} ${isColHovered ? styles.colHovered : ""}`}
                    onMouseEnter={() => {
                      setHoveredRow(null);
                      setHoveredCol(defType);
                    }}
                    title={`Selected Defender: ${defType} (Click to remove)`}
                  >
                    <button
                      type="button"
                      className={`${styles.typeHeaderBtn} ${styles.selectedDefenderBtn}`}
                      onClick={() => handleToggleDefender(defType)}
                      aria-label={`Deselect ${defType} defender`}
                      aria-pressed={true}
                    >
                      <TypeBadge type={defType} square size="md" />
                    </button>
                  </th>
                );
              })}

              {/* Remaining Unselected Defender Columns */}
              {remainingTypes.map((defType) => {
                const isColHovered = hoveredCol === defType;

                return (
                  <th
                    key={defType}
                    className={`${styles.thDefender} ${isColHovered ? styles.colHovered : ""}`}
                    onMouseEnter={() => {
                      setHoveredRow(null);
                      setHoveredCol(defType);
                    }}
                    title={`Defending: ${defType} (Click to select)`}
                  >
                    <button
                      type="button"
                      className={`${styles.typeHeaderBtn} ${hasSelection ? styles.headerFaded : ""}`}
                      onClick={() => handleToggleDefender(defType)}
                      aria-label={`Select ${defType} defender`}
                      aria-pressed={false}
                    >
                      <TypeBadge type={defType} square size="md" />
                    </button>
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {ALL_POKEMON_TYPES.map((atk) => {
              const isRowHovered = hoveredRow === atk;

              return (
                <tr key={atk}>
                  <th
                    className={`${styles.thAttacker} ${isRowHovered ? styles.rowHovered : ""}`}
                    onMouseEnter={() => {
                      setHoveredRow(atk);
                      setHoveredCol(null);
                    }}
                    title={`Attacking: ${atk}`}
                  >
                    <TypeBadge type={atk} square size="md" />
                  </th>

                  {/* If defenders are selected: One combined cell spanning all selected columns */}
                  {hasSelection && (() => {
                    const mult = combinedMultipliers?.[atk] ?? 1;
                    const text = formatMultiplierValue(mult, true);
                    const multClass = getMultiplierClass(mult);
                    const isFraction = mult < 1 && mult > 0;
                    const isCombinedColHovered = selectedDefenders.some((d) => hoveredCol === d);
                    const isCellHovered = isRowHovered && isCombinedColHovered;

                    return (
                      <td
                        colSpan={selectedDefenders.length}
                        className={`${styles.tdCell} ${styles.cellCombined} ${multClass} ${isRowHovered ? styles.rowHovered : ""} ${isCellHovered ? styles.cellHovered : ""}`}
                        onMouseEnter={() => {
                          setHoveredRow(atk);
                          setHoveredCol(selectedDefenders[0]);
                        }}
                        title={formatEffectivenessComparison(atk, selectedDefenders, mult)}
                        aria-label={formatEffectivenessComparison(atk, selectedDefenders, mult)}
                      >
                        {text && (
                          <span
                            className={`${styles.cellNumber} ${isFraction ? styles.cellFraction : ""} ${mult === 1 ? styles.neutralNumber : ""}`}
                          >
                            {text}
                          </span>
                        )}
                      </td>
                    );
                  })()}

                  {/* Remaining unselected columns */}
                  {remainingTypes.map((defType) => {
                    const isColHovered = hoveredCol === defType;
                    const isCellHovered = isRowHovered && isColHovered;
                    const mult = getEffectiveness(typeChart, atk, defType);
                    const text = formatMultiplierValue(mult, false);
                    const multClass = getMultiplierClass(mult);
                    const isFraction = mult < 1 && mult > 0;

                    return (
                      <td
                        key={defType}
                        className={`${styles.tdCell} ${hasSelection ? styles.cellFaded : ""} ${multClass} ${isRowHovered ? styles.rowHovered : ""} ${isColHovered ? styles.colHovered : ""} ${isCellHovered ? styles.cellHovered : ""}`}
                        onMouseEnter={() => {
                          setHoveredRow(atk);
                          setHoveredCol(defType);
                        }}
                        title={formatEffectivenessComparison(atk, defType, mult)}
                        aria-label={formatEffectivenessComparison(atk, defType, mult)}
                      >
                        {text && (
                          <span
                            className={`${styles.cellNumber} ${isFraction ? styles.cellFraction : ""}`}
                          >
                            {text}
                          </span>
                        )}
                      </td>
                    );
                  })}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

