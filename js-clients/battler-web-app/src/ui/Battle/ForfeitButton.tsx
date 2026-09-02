import { useEffect, useState } from "react";

interface ForfeitButtonProps {
  onForfeit: () => void;
  isLoading: boolean;
}

export default function ForfeitButton({ onForfeit, isLoading }: ForfeitButtonProps) {
  const [showForfeitConfirm, setShowForfeitConfirm] = useState(false);

  // Reset forfeit confirm when loading finishes
  useEffect(() => {
    if (!isLoading) {
      setShowForfeitConfirm(false);
    }
  }, [isLoading]);

  // Reset forfeit confirmation after 4 seconds of inactivity
  useEffect(() => {
    if (showForfeitConfirm) {
      const timer = setTimeout(() => {
        setShowForfeitConfirm(false);
      }, 4000);
      return () => clearTimeout(timer);
    }
  }, [showForfeitConfirm]);

  const handleForfeitClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowForfeitConfirm(true);
  };

  const handleForfeitCancel = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowForfeitConfirm(false);
  };

  const handleForfeitConfirm = (e: React.MouseEvent) => {
    e.stopPropagation();
    onForfeit();
  };

  if (showForfeitConfirm) {
    return (
      <div className="flex-row gap-xs align-center">
        <button className="btn btn-sm btn-danger" onClick={handleForfeitConfirm} disabled={isLoading}>
          Confirm
        </button>
        <button className="btn btn-sm btn-secondary" onClick={handleForfeitCancel} disabled={isLoading}>
          Cancel
        </button>
      </div>
    );
  }

  return (
    <button className="btn btn-sm btn-danger" onClick={handleForfeitClick} disabled={isLoading}>
      Forfeit
    </button>
  );
}
