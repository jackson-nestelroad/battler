import {
  type CSSProperties,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type ReactNode,
  useContext,
  useEffect,
  useId,
  useRef,
  useState,
} from "react";
import type { DescriptionData, ResourceData } from "battler-data-service-client";
import {
  type ResourceType,
  useGenericResource,
  useResourceData,
} from "../../../hooks/useDataStore";
import { getElementRect } from "../../../utils/floatingCoords";
import { isTargetInsideModal } from "../../../utils/dom";
import AbilityTooltipCard from "./AbilityTooltipCard";
import ConditionTooltipCard from "./ConditionTooltipCard";
import cardStyles from "./DataTooltipCard.module.scss";
import styles from "./DataTooltipTrigger.module.scss";
import FloatingTooltip from "./FloatingTooltip";
import ItemTooltipCard from "./ItemTooltipCard";
import MoveTooltipCard from "./MoveTooltipCard";
import SpeciesTooltipCard from "./SpeciesTooltipCard";
import { TooltipParentContext, useTooltipChildTracker } from "./TooltipContext";

export type DataResourceType = ResourceType | "resource";

export interface DataTooltipTriggerProps {
  resourceType: DataResourceType;
  name?: string | null;
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
  as?: "span" | "div" | "button";
  preferredPlacement?: "top" | "bottom" | "left" | "right";
  showUnderline?: boolean;
  ariaLabel?: string;
  title?: string;
  onClick?: (e: ReactMouseEvent<HTMLElement> | ReactKeyboardEvent<HTMLElement>) => void;
}

function LoadingCard() {
  return (
    <div
      className={`${cardStyles.card} ${cardStyles.cardFixed} ${cardStyles.loadingCard}`}
      role="status"
      aria-live="polite"
    >
      <span className="spinner spinner-sm" />
      <span>Loading...</span>
    </div>
  );
}

function EmptyCard({ name }: { name: string }) {
  return (
    <div
      className={`${cardStyles.card} ${cardStyles.cardFixed} ${cardStyles.emptyCard}`}
      role="status"
      aria-live="polite"
    >
      <span className={cardStyles.emptyText}>No data available for "{name}"</span>
    </div>
  );
}

function renderResourceCard(
  data: ResourceData,
  fallbackName: string,
  description?: DescriptionData | null,
) {
  switch (data.type) {
    case "move":
      return <MoveTooltipCard data={data.data} description={description} />;
    case "ability":
      return <AbilityTooltipCard data={data.data} description={description} />;
    case "item":
      return <ItemTooltipCard data={data.data} description={description} />;
    case "condition":
      return <ConditionTooltipCard data={data.data} description={description} />;
    case "species":
      return <SpeciesTooltipCard data={data.data} description={description} />;
    default:
      return <EmptyCard name={fallbackName} />;
  }
}

function TypedResourceContent({
  type,
  name,
}: {
  type: ResourceType;
  name: string;
}) {
  const { data, description, loading } = useResourceData(type, name);
  if (loading) return <LoadingCard />;
  if (!data) return <EmptyCard name={name} />;
  return renderResourceCard({ type, data } as ResourceData, name, description);
}

function GenericResourceContent({ name }: { name: string }) {
  const { data, description, loading } = useGenericResource(name);
  if (loading) return <LoadingCard />;
  if (!data) return <EmptyCard name={name} />;
  return renderResourceCard(data, name, description);
}

function ResourceContent({
  resourceType,
  name,
}: {
  resourceType: DataResourceType;
  name: string;
}) {
  if (resourceType === "condition" || resourceType === "resource") {
    return <GenericResourceContent name={name} />;
  }
  return <TypedResourceContent type={resourceType} name={name} />;
}

export default function DataTooltipTrigger({
  resourceType,
  name,
  children,
  className = "",
  style,
  as = "span",
  preferredPlacement = "top",
  showUnderline = true,
  ariaLabel,
  title,
  onClick,
}: DataTooltipTriggerProps) {
  const triggerId = useId();
  const [isOpen, setIsOpen] = useState(false);
  const [targetRect, setTargetRect] = useState<DOMRect | null>(null);
  const triggerRef = useRef<HTMLElement | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);

  const parentContext = useContext(TooltipParentContext);
  const { openChildCount, isTargetInChild, closeChild, contextValue } =
    useTooltipChildTracker(parentContext);

  useEffect(() => {
    if (!isOpen) return;
    const unregisterContent =
      contentRef.current && parentContext?.registerChildContent
        ? parentContext.registerChildContent(contentRef.current)
        : undefined;
    const unregisterOpen = parentContext?.registerChildOpen?.();
    const unregisterActiveChild = parentContext?.openChild?.(triggerId, () => {
      setIsOpen(false);
    });
    return () => {
      unregisterContent?.();
      unregisterOpen?.();
      unregisterActiveChild?.();
    };
  }, [isOpen, parentContext, triggerId]);

  useEffect(() => {
    if (!isOpen) {
      closeChild();
    }
  }, [isOpen, closeChild]);

  const cleanName = name?.trim();

  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (e: PointerEvent) => {
      const target = e.target as Node | null;
      if (!target) return;
      if (triggerRef.current?.contains(target)) return;

      // Ignore clicks inside active dialog overlays / modals (e.g. FxLangModal)
      if (isTargetInsideModal(target)) {
        return;
      }

      // If clicked inside our own content (the parent card):
      if (contentRef.current?.contains(target)) {
        // If clicked on parent surface rather than an active child tooltip, dismiss open child
        if (!isTargetInChild(target)) {
          closeChild();
        }
        return;
      }

      // If clicked inside an open child tooltip portal, stay open
      if (isTargetInChild(target)) return;

      // Clicked outside both our content and any children
      closeChild();
      setIsOpen(false);
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (openChildCount > 0) {
          closeChild();
          return;
        }
        setIsOpen(false);
      }
    };

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen, isTargetInChild, openChildCount, closeChild]);

  if (!cleanName) {
    const Component = as;
    return (
      <Component
        type={as === "button" ? "button" : undefined}
        className={className}
        style={style}
        onClick={onClick}
        aria-label={ariaLabel}
        title={title}
      >
        {children}
      </Component>
    );
  }

  const toggleTooltip = (target: HTMLElement) => {
    setTargetRect(getElementRect(target));
    setIsOpen((prev) => !prev);
  };

  const handleClick = (e: ReactMouseEvent<HTMLElement>) => {
    onClick?.(e);
    if (e.defaultPrevented) return;

    e.stopPropagation();
    toggleTooltip(e.currentTarget);
  };

  const handleTriggerKeyDown = (e: ReactKeyboardEvent<HTMLElement>) => {
    if (as !== "button" && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      e.stopPropagation();
      onClick?.(e);
      if (e.defaultPrevented) return;
      toggleTooltip(e.currentTarget);
    }
  };

  const displayClass =
    as === "div" ? styles.triggerBlock : styles.triggerInline;

  const triggerClasses = [
    styles.trigger,
    displayClass,
    showUnderline && styles.triggerUnderline,
    className,
  ]
    .filter(Boolean)
    .join(" ");

  const Component = as;
  const sharedAriaProps = {
    "aria-label": ariaLabel,
    title,
    "aria-haspopup": "dialog" as const,
    "aria-expanded": isOpen,
  };

  const elementProps =
    as === "button"
      ? {
          type: "button" as const,
          ...sharedAriaProps,
        }
      : {
          role: "button",
          tabIndex: 0,
          onKeyDown: handleTriggerKeyDown,
          ...sharedAriaProps,
        };

  return (
    <>
      <Component
        ref={triggerRef as React.Ref<never>}
        className={triggerClasses}
        style={style}
        onClick={handleClick}
        {...elementProps}
      >
        {children}
      </Component>

      <FloatingTooltip
        isOpen={isOpen}
        targetRect={targetRect}
        targetRef={triggerRef}
        containerRef={contentRef}
        preferredPlacement={preferredPlacement}
      >
        <TooltipParentContext.Provider value={contextValue}>
          <ResourceContent resourceType={resourceType} name={cleanName} />
        </TooltipParentContext.Provider>
      </FloatingTooltip>
    </>
  );
}
