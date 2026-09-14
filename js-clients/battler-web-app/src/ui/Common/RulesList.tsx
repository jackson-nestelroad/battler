import { getRuleBadgeClass } from "../../utils/battle";

interface RulesListProps {
  rules: string[];
}

export default function RulesList({ rules }: RulesListProps) {
  return (
    <div className="flex-row flex-wrap gap-xs">
      {rules.map((rule, idx) => (
        <span key={idx} className={`badge ${getRuleBadgeClass(rule)}`}>
          {rule}
        </span>
      ))}
    </div>
  );
}
