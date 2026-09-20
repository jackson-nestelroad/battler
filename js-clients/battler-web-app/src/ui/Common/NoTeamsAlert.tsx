import { selectBattle } from "../../store/battlesSlice";
import { useAppDispatch } from "../../store/store";

interface NoTeamsAlertProps {
  className?: string;
}

export default function NoTeamsAlert({ className = "" }: NoTeamsAlertProps) {
  const dispatch = useAppDispatch();

  const handleNavigateToTeams = () => {
    dispatch(selectBattle({ view: "teams", battleId: null }));
  };

  return (
    <div
      role="alert"
      className={`alert alert-warning flex-row align-center justify-between gap-s flex-wrap w-full mb-0 ${className}`.trim()}
    >
      <span className="alert-message">No teams configured</span>
      <button
        type="button"
        className="btn btn-secondary btn-sm"
        onClick={handleNavigateToTeams}
      >
        Go to Teams
      </button>
    </div>
  );
}
