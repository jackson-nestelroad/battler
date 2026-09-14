interface ErrorBannerProps {
  message: string | null;
  details?: string[] | null;
  onClear?: () => void;
}

export default function ErrorBanner({ message, details, onClear }: ErrorBannerProps) {
  if (!message) return null;

  return (
    <div className="alert">
      <div className="alert-message">
        <div>{message}</div>
        {details && details.length > 0 && (
          <ul className="alert-details">
            {details.map((detail, idx) => (
              <li key={idx} className="alert-detail-item">
                {detail}
              </li>
            ))}
          </ul>
        )}
      </div>
      {onClear && (
        <button onClick={onClear} className="alert-close" aria-label="Clear error">
          ✕
        </button>
      )}
    </div>
  );
}
