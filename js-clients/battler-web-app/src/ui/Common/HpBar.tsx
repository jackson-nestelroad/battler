interface HpBarProps {
  hp: number;
  maxHp: number;
}

export default function HpBar({ hp, maxHp }: HpBarProps) {
  const percent = maxHp > 0 ? Math.max(0, Math.min(100, (hp / maxHp) * 100)) : 0;
  const hpClass = percent < 20 ? "hp-red" : percent < 50 ? "hp-yellow" : "hp-green";

  return (
    <div className="hp-bar-container">
      <div className={`hp-bar ${hpClass}`} style={{ width: `${percent}%` }} />
    </div>
  );
}
