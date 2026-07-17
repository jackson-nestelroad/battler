interface ErrorBannerProps {
  message: string | null;
  onClear?: () => void;
}

export default function ErrorBanner({ message, onClear }: ErrorBannerProps) {
  if (!message) return null;

  return (
    <div className="alert">
      <div className="alert-message">{message}</div>
      {onClear && (
        <button onClick={onClear} className="alert-close" aria-label="Clear error">
          ✕
        </button>
      )}
    </div>
  );
}

