import {
  type CSSProperties,
  type MouseEvent as ReactMouseEvent,
  type ReactNode,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  useAbilityData,
  useGenericResource,
  useItemData,
  useMoveData,
  useSpeciesData,
} from "../../../hooks/useDataStore";
import AbilityTooltipCard from "./AbilityTooltipCard";
import ConditionTooltipCard from "./ConditionTooltipCard";
import styles from "./DataTooltipTrigger.module.scss";
import FloatingTooltip from "./FloatingTooltip";
import ItemTooltipCard from "./ItemTooltipCard";
import MoveTooltipCard from "./MoveTooltipCard";
import SpeciesTooltipCard from "./SpeciesTooltipCard";

export type DataResourceType =
  | "move"
  | "ability"
  | "item"
  | "condition"
  | "species"
  | "resource";

export interface DataTooltipTriggerProps {
  resourceType: DataResourceType;
  name?: string | null;
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
  as?: "span" | "div" | "button";
  preferredPlacement?: "top" | "bottom" | "left" | "right";
  showUnderline?: boolean;
  onClick?: (e: ReactMouseEvent<HTMLElement>) => void;
}

function LoadingCard() {
  return (
    <div className={styles.loadingCard}>
      <span className="spinner spinner-sm" />
      <span>Loading...</span>
    </div>
  );
}

function EmptyCard({ name }: { name: string }) {
  return (
    <div className={styles.emptyCard}>
      <span className={styles.emptyText}>No data available for "{name}"</span>
    </div>
  );
}

function MoveContent({ name }: { name: string }) {
  const { data, loading } = useMoveData(name);
  if (loading) return <LoadingCard />;
  if (!data) return <EmptyCard name={name} />;
  return <MoveTooltipCard data={data} />;
}

function AbilityContent({ name }: { name: string }) {
  const { data, loading } = useAbilityData(name);
  if (loading) return <LoadingCard />;
  if (!data) return <EmptyCard name={name} />;
  return <AbilityTooltipCard data={data} />;
}

function ItemContent({ name }: { name: string }) {
  const { data, loading } = useItemData(name);
  if (loading) return <LoadingCard />;
  if (!data) return <EmptyCard name={name} />;
  return <ItemTooltipCard data={data} />;
}

const CONDITION_PRIORITY = {
  priority: ["condition", "move", "ability", "item"] as const,
};

function ConditionContent({ name }: { name: string }) {
  const { data, loading } = useGenericResource(name, CONDITION_PRIORITY);
  if (loading) return <LoadingCard />;
  if (!data) return <EmptyCard name={name} />;
  switch (data.type) {
    case "move":
      return <MoveTooltipCard data={data.data} />;
    case "ability":
      return <AbilityTooltipCard data={data.data} />;
    case "item":
      return <ItemTooltipCard data={data.data} />;
    case "condition":
      return <ConditionTooltipCard data={data.data} />;
    default:
      return <EmptyCard name={name} />;
  }
}

function GenericResourceContent({ name }: { name: string }) {
  const { data, loading } = useGenericResource(name);
  if (loading) return <LoadingCard />;
  if (!data) return <EmptyCard name={name} />;
  switch (data.type) {
    case "move":
      return <MoveTooltipCard data={data.data} />;
    case "ability":
      return <AbilityTooltipCard data={data.data} />;
    case "item":
      return <ItemTooltipCard data={data.data} />;
    case "condition":
      return <ConditionTooltipCard data={data.data} />;
    case "species":
      return <SpeciesTooltipCard data={data.data} />;
    default:
      return <EmptyCard name={name} />;
  }
}

function SpeciesContent({ name }: { name: string }) {
  const { data, loading } = useSpeciesData(name);
  if (loading) return <LoadingCard />;
  if (!data) return <EmptyCard name={name} />;
  return <SpeciesTooltipCard data={data} />;
}

function ResourceContent({
  resourceType,
  name,
}: {
  resourceType: DataResourceType;
  name: string;
}) {
  switch (resourceType) {
    case "move":
      return <MoveContent name={name} />;
    case "ability":
      return <AbilityContent name={name} />;
    case "item":
      return <ItemContent name={name} />;
    case "condition":
      return <ConditionContent name={name} />;
    case "species":
      return <SpeciesContent name={name} />;
    case "resource":
      return <GenericResourceContent name={name} />;
  }
}

interface TooltipParentContextValue {
  registerChildContent: (el: HTMLElement) => () => void;
}

const TooltipParentContext = createContext<TooltipParentContextValue | null>(null);

export default function DataTooltipTrigger({
  resourceType,
  name,
  children,
  className = "",
  style,
  as = "span",
  preferredPlacement = "top",
  showUnderline = true,
  onClick,
}: DataTooltipTriggerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [targetRect, setTargetRect] = useState<DOMRect | null>(null);
  const triggerRef = useRef<HTMLElement | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);

  const parentContext = useContext(TooltipParentContext);
  const childContentEls = useRef<Set<HTMLElement>>(new Set());

  const registerChildContent = useCallback(
    (el: HTMLElement) => {
      childContentEls.current.add(el);
      const unregisterFromParent = parentContext?.registerChildContent(el);
      return () => {
        childContentEls.current.delete(el);
        unregisterFromParent?.();
      };
    },
    [parentContext],
  );

  const contextValue = useMemo(
    () => ({ registerChildContent }),
    [registerChildContent],
  );

  useEffect(() => {
    if (isOpen && contentRef.current && parentContext) {
      return parentContext.registerChildContent(contentRef.current);
    }
  }, [isOpen, parentContext]);

  const cleanName = name?.trim();

  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (e: PointerEvent) => {
      const target = e.target as Node | null;
      if (!target) return;
      if (triggerRef.current?.contains(target)) return;
      if (contentRef.current?.contains(target)) return;
      for (const childEl of childContentEls.current) {
        if (childEl.contains(target)) return;
      }
      setIsOpen(false);
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setIsOpen(false);
      }
    };

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen]);

  if (!cleanName) {
    const Component = as;
    return (
      <Component className={className} style={style}>
        {children}
      </Component>
    );
  }

  const handleClick = (e: ReactMouseEvent<HTMLElement>) => {
    onClick?.(e);
    if (e.defaultPrevented) return;

    e.stopPropagation();
    const el = e.currentTarget;
    let rect = el.getBoundingClientRect();
    if (rect.width === 0 && rect.height === 0 && el.firstElementChild) {
      rect = (el.firstElementChild as HTMLElement).getBoundingClientRect();
    }
    setTargetRect(rect);
    setIsOpen((prev) => !prev);
  };

  const handleTriggerKeyDown = (e: React.KeyboardEvent<HTMLElement>) => {
    if (as !== "button" && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      e.stopPropagation();
      const el = e.currentTarget;
      let rect = el.getBoundingClientRect();
      if (rect.width === 0 && rect.height === 0 && el.firstElementChild) {
        rect = (el.firstElementChild as HTMLElement).getBoundingClientRect();
      }
      setTargetRect(rect);
      setIsOpen((prev) => !prev);
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
  const elementProps =
    as === "button"
      ? { type: "button" as const }
      : { role: "button", tabIndex: 0, onKeyDown: handleTriggerKeyDown };

  return (
    <>
      <Component
        ref={triggerRef as any}
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
        preferredPlacement={preferredPlacement}
      >
        <div ref={contentRef}>
          <TooltipParentContext.Provider value={contextValue}>
            <ResourceContent resourceType={resourceType} name={cleanName} />
          </TooltipParentContext.Provider>
        </div>
      </FloatingTooltip>
    </>
  );
}
