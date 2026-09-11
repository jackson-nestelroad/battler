import { useEffect, type ReactNode } from "react";
import { createPortal } from "react-dom";
import styles from "./Modal.module.scss";

export interface ModalProps {
  isOpen?: boolean;
  onClose: () => void;
  title: ReactNode;
  subtitle?: string;
  headerLeft?: ReactNode;
  headerActions?: ReactNode;
  maxWidth?: "sm" | "md" | "lg" | "xl";
  className?: string;
  contentClassName?: string;
  contentRole?: string;
  contentAriaLabel?: string;
  ariaLabelledBy?: string;
  children: ReactNode;
}

export default function Modal({
  isOpen = true,
  onClose,
  title,
  subtitle,
  headerLeft,
  headerActions,
  maxWidth = "md",
  className = "",
  contentClassName = "",
  contentRole,
  contentAriaLabel,
  ariaLabelledBy,
  children,
}: ModalProps) {
  const titleId =
    ariaLabelledBy ||
    (typeof title === "string"
      ? `modal-${title.toLowerCase().replace(/\s+/g, "-")}`
      : "modal-title");

  // Dismiss on Escape key (capture phase so background elements don't catch it first)
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [isOpen, onClose]);

  if (!isOpen || typeof document === "undefined") {
    return null;
  }

  const maxWidthClass =
    maxWidth === "sm"
      ? styles.modalSm
      : maxWidth === "lg"
        ? styles.modalLg
        : maxWidth === "xl"
          ? styles.modalXl
          : styles.modalMd;

  return createPortal(
    <div
      className={styles.backdrop}
      onClick={onClose}
      onPointerDown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
    >
      <div
        className={`${styles.modal} ${maxWidthClass} ${className}`.trim()}
        onClick={(e) => e.stopPropagation()}
        onPointerDown={(e) => e.stopPropagation()}
      >
        <header className={styles.modalHeader}>
          <div className={styles.titleWrapper}>
            {headerLeft}
            {typeof title === "string" ? (
              <h2 id={titleId} className={styles.title}>
                {title}
              </h2>
            ) : (
              title
            )}
            {subtitle && <span className={styles.subtitle}>{subtitle}</span>}
          </div>

          <div className="flex-row align-center gap-s">
            {headerActions}
            <button
              type="button"
              aria-label="Close"
              title="Close"
              className={styles.closeBtn}
              onClick={onClose}
            >
              ✕
            </button>
          </div>
        </header>

        <div
          className={`${styles.content} ${contentClassName}`.trim()}
          role={contentRole}
          aria-label={contentAriaLabel}
        >
          {children}
        </div>
      </div>
    </div>,
    document.body,
  );
}

