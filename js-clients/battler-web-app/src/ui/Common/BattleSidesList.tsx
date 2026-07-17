import PlayerStatusRow from "./PlayerStatusRow";

interface PlayerViewData {
  id: string;
  name: string;
  status?: string | null;
  state?: string;
}

interface SideViewData {
  name: string;
  players: PlayerViewData[];
}

interface BattleSidesListProps {
  sides: SideViewData[];
  isProposal?: boolean; // If true, formats as proposal status
}

export default function BattleSidesList({ sides, isProposal = false }: BattleSidesListProps) {
  return (
    <div className="flex-col gap-s">
      {sides.map((side, sIdx) => {
        const sideTitle = isProposal ? `Side ${sIdx + 1} (${side.name})` : side.name;
        return (
          <div key={sIdx} className="side-block">
            <span className="side-block-title">{sideTitle}</span>
            <div className="flex-col gap-xs">
              {side.players.map((p) => {
                let statusText = "";
                let badgeClass = "";

                if (isProposal) {
                  statusText = p.status || "Pending";
                  badgeClass =
                    p.status === "accepted"
                      ? "badge-success"
                      : p.status === "rejected"
                      ? "badge-danger"
                      : "badge-warning";
                } else {
                  const isReady = p.state === "ready";
                  statusText = isReady ? "Ready" : "Preparing";
                  badgeClass = isReady ? "badge-success" : "badge-warning";
                }

                return (
                  <PlayerStatusRow
                    key={p.id}
                    name={p.name}
                    status={statusText}
                    badgeClass={badgeClass}
                  />
                );
              })}
            </div>
          </div>
        );
      })}
    </div>
  );
}
