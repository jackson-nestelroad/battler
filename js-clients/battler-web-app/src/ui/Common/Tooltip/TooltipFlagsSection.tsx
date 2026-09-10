import cardStyles from "./DataTooltipCard.module.scss";

export interface TooltipFlagsSectionProps {
  flags?: Iterable<string> | null;
}

export default function TooltipFlagsSection({ flags }: TooltipFlagsSectionProps) {
  const sortedFlags = flags ? Array.from(new Set(flags)).sort() : [];
  if (sortedFlags.length === 0) return null;

  return (
    <section className="flex-col gap-xxs">
      <span className={cardStyles.sectionTitle}>Flags</span>
      <div className="flex-row flex-wrap gap-xxs">
        {sortedFlags.map((flag) => (
          <span key={flag} className={cardStyles.flagBadge}>
            {flag}
          </span>
        ))}
      </div>
    </section>
  );
}
