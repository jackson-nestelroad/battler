import {
  type KeyboardEvent,
  type ReactNode,
  useEffect,
  useId,
  useRef,
  useState,
} from "react";
import styles from "./CustomSelect.module.scss";

export interface CustomSelectOption<T = string> {
  value: T;
  label: string;
  endContent?: ReactNode;
  disabled?: boolean;
}

export interface CustomSelectProps<T = string> {
  id?: string;
  className?: string;
  value?: T;
  onChange: (value: T) => void;
  options: CustomSelectOption<T>[];
  placeholder?: string;
  disabled?: boolean;
  required?: boolean;
  ariaLabel?: string;
  renderTrigger?: (selectedOption?: CustomSelectOption<T>) => ReactNode;
  renderOption?: (
    option: CustomSelectOption<T>,
    isSelected: boolean,
    isHighlighted: boolean,
  ) => ReactNode;
}

export default function CustomSelect<T = string>({
  id,
  className,
  value,
  onChange,
  options,
  placeholder = "Select...",
  disabled = false,
  required = false,
  ariaLabel,
  renderTrigger,
  renderOption,
}: CustomSelectProps<T>) {
  const [isOpen, setIsOpen] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(-1);

  const containerRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const optionRefs = useRef<(HTMLDivElement | null)[]>([]);

  const generatedId = useId();
  const selectId = id || generatedId;
  const listboxId = `${selectId}-listbox`;

  const selectedOption = options.find((opt) => opt.value === value);

  const handleOpen = () => {
    if (disabled || options.length === 0) return;
    const currentIndex = options.findIndex((opt) => opt.value === value);
    setHighlightedIndex(currentIndex >= 0 ? currentIndex : 0);
    setIsOpen(true);
  };

  const handleClose = () => {
    setIsOpen(false);
    setHighlightedIndex(-1);
  };

  const handleSelect = (optionValue: T) => {
    onChange(optionValue);
    handleClose();
    triggerRef.current?.focus();
  };

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

  useEffect(() => {
    if (isOpen && highlightedIndex >= 0) {
      optionRefs.current[highlightedIndex]?.scrollIntoView({
        block: "nearest",
      });
    }
  }, [isOpen, highlightedIndex]);

  const handleKeyDown = (e: KeyboardEvent<HTMLButtonElement>) => {
    if (disabled || options.length === 0) return;

    switch (e.key) {
      case "ArrowDown": {
        e.preventDefault();
        if (!isOpen) {
          handleOpen();
        } else {
          setHighlightedIndex((prev) => {
            let next = prev < options.length - 1 ? prev + 1 : 0;
            while (options[next]?.disabled && next !== prev) {
              next = next < options.length - 1 ? next + 1 : 0;
            }
            return next;
          });
        }
        break;
      }
      case "ArrowUp": {
        e.preventDefault();
        if (!isOpen) {
          handleOpen();
        } else {
          setHighlightedIndex((prev) => {
            let next = prev > 0 ? prev - 1 : options.length - 1;
            while (options[next]?.disabled && next !== prev) {
              next = next > 0 ? next - 1 : options.length - 1;
            }
            return next;
          });
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
          highlightedIndex < options.length &&
          !options[highlightedIndex]?.disabled
        ) {
          handleSelect(options[highlightedIndex].value);
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

  const hiddenInputValue =
    typeof value === "string" || typeof value === "number"
      ? String(value)
      : selectedOption
        ? selectedOption.label
        : "";

  const resolvedAriaLabel =
    ariaLabel ||
    (selectedOption
      ? `Selected: ${selectedOption.label}`
      : placeholder);

  return (
    <div
      ref={containerRef}
      className={`${styles.container} ${className || ""}`.trim()}
    >
      <input
        type="hidden"
        name={id}
        value={hiddenInputValue}
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
        aria-label={resolvedAriaLabel}
      >
        <div className={styles.triggerContent}>
          {renderTrigger ? (
            renderTrigger(selectedOption)
          ) : selectedOption ? (
            <>
              <span className={styles.label} title={selectedOption.label}>
                {selectedOption.label}
              </span>
              {selectedOption.endContent}
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
          aria-label={ariaLabel || "Options"}
        >
          {options.map((opt, idx) => {
            const isOptionSelected = opt.value === value;
            const isHighlighted = idx === highlightedIndex;

            return (
              <div
                key={String(opt.value)}
                ref={(el) => {
                  optionRefs.current[idx] = el;
                }}
                role="option"
                aria-selected={isOptionSelected}
                aria-disabled={opt.disabled}
                className={`${styles.option} ${isHighlighted ? styles.highlighted : ""} ${isOptionSelected ? styles.selected : ""} ${opt.disabled ? styles.disabled : ""}`}
                onClick={() => {
                  if (!opt.disabled) {
                    handleSelect(opt.value);
                  }
                }}
                onMouseEnter={() => {
                  if (!opt.disabled) {
                    setHighlightedIndex(idx);
                  }
                }}
              >
                {renderOption ? (
                  renderOption(opt, isOptionSelected, isHighlighted)
                ) : (
                  <>
                    <span className={styles.optionLabel} title={opt.label}>
                      {opt.label}
                    </span>
                    {opt.endContent}
                  </>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
