interface PlayerStatusRowProps {
  name: string;
  status: string;
  badgeClass: string;
}

export default function PlayerStatusRow({ name, status, badgeClass }: PlayerStatusRowProps) {
  return (
    <div className="flex-row justify-between align-center w-full">
      <span>@{name}</span>
      <span className={`badge ${badgeClass}`}>
        {status}
      </span>
    </div>
  );
}
