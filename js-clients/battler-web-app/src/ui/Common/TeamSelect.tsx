import {
  type KeyboardEvent,
  useEffect,
  useId,
  useRef,
  useState,
} from "react";
import type { MonData } from "battler-types";
import TeamMonIcons from "./TeamMonIcons";
import styles from "./TeamSelect.module.scss";

export interface TeamSelectProps {
  id?: string;
  className?: string;
  value: string;
  onChange: (value: string) => void;
  teamNames: string[];
  teams: Record<string, MonData[] | Partial<MonData>[]>;
  disabled?: boolean;
  required?: boolean;
  placeholder?: string;
}

export default function TeamSelect({
  id,
  className,
  value,
  onChange,
  teamNames,
  teams,
  disabled = false,
  required = false,
  placeholder = "Select team...",
}: TeamSelectProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(-1);

  const containerRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const optionRefs = useRef<(HTMLDivElement | null)[]>([]);

  const generatedId = useId();
  const selectId = id || generatedId;
  const listboxId = `${selectId}-listbox`;

  const isValidValue = Boolean(value && teamNames.includes(value));
  const selectedMembers = isValidValue ? teams[value] : undefined;

  // Sync highlighted index when opening
  const handleOpen = () => {
    if (disabled || teamNames.length === 0) return;
    const currentIndex = teamNames.indexOf(value);
    setHighlightedIndex(currentIndex >= 0 ? currentIndex : 0);
    setIsOpen(true);
  };

  const handleClose = () => {
    setIsOpen(false);
    setHighlightedIndex(-1);
  };

  const handleSelect = (teamName: string) => {
    onChange(teamName);
    handleClose();
    triggerRef.current?.focus();
  };

  // Close on click outside
  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        handleClose();
      }
    };

    document.addEventListener("mousedown", handlePointerDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
    };
  }, [isOpen]);

  // Scroll active option into view during keyboard navigation
  useEffect(() => {
    if (isOpen && highlightedIndex >= 0) {
      optionRefs.current[highlightedIndex]?.scrollIntoView({
        block: "nearest",
      });
    }
  }, [isOpen, highlightedIndex]);

  const handleKeyDown = (e: KeyboardEvent<HTMLButtonElement>) => {
    if (disabled || teamNames.length === 0) return;

    switch (e.key) {
      case "ArrowDown": {
        e.preventDefault();
        if (!isOpen) {
          handleOpen();
        } else {
          setHighlightedIndex((prev) =>
            prev < teamNames.length - 1 ? prev + 1 : 0,
          );
        }
        break;
      }
      case "ArrowUp": {
        e.preventDefault();
        if (!isOpen) {
          handleOpen();
        } else {
          setHighlightedIndex((prev) =>
            prev > 0 ? prev - 1 : teamNames.length - 1,
          );
        }
        break;
      }
      case "Enter":
      case " ": {
        e.preventDefault();
        if (!isOpen) {
          handleOpen();
        } else if (
          highlightedIndex >= 0 &&
          highlightedIndex < teamNames.length
        ) {
          handleSelect(teamNames[highlightedIndex]);
        }
        break;
      }
      case "Escape": {
        if (isOpen) {
          e.preventDefault();
          handleClose();
        }
        break;
      }
      case "Tab": {
        if (isOpen) {
          handleClose();
        }
        break;
      }
      default:
        break;
    }
  };

  return (
    <div
      ref={containerRef}
      className={`${styles.container} ${className || ""}`.trim()}
    >
      <input
        type="hidden"
        name={id}
        value={isValidValue ? value : ""}
        required={required}
      />
      <button
        type="button"
        id={selectId}
        ref={triggerRef}
        className={`${styles.trigger} ${isOpen ? styles.open : ""}`}
        onClick={() => (isOpen ? handleClose() : handleOpen())}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        role="combobox"
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        aria-controls={listboxId}
        aria-required={required}
        aria-label={isValidValue ? `Selected team: ${value}` : placeholder}
      >
        <div className={styles.triggerContent}>
          {isValidValue ? (
            <>
              <span className={styles.teamName} title={value}>
                {value}
              </span>
              {selectedMembers && selectedMembers.length > 0 && (
                <TeamMonIcons members={selectedMembers} size="md" />
              )}
            </>
          ) : (
            <span className={styles.placeholder}>{placeholder}</span>
          )}
        </div>
        <svg
          className={`${styles.chevron} ${isOpen ? styles.open : ""}`}
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>

      {isOpen && (
        <div
          id={listboxId}
          role="listbox"
          className={styles.menu}
          aria-label="Teams"
        >
          {teamNames.map((name, idx) => {
            const isSelected = value === name;
            const isHighlighted = idx === highlightedIndex;
            const members = teams[name];

            return (
              <div
                key={name}
                ref={(el) => {
                  optionRefs.current[idx] = el;
                }}
                role="option"
                aria-selected={isSelected}
                className={`${styles.option} ${isHighlighted ? styles.highlighted : ""} ${isSelected ? styles.selected : ""}`}
                onClick={() => handleSelect(name)}
                onMouseEnter={() => setHighlightedIndex(idx)}
              >
                <span className={styles.optionName} title={name}>
                  {name}
                </span>
                {members && members.length > 0 && (
                  <TeamMonIcons members={members} size="md" />
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}


