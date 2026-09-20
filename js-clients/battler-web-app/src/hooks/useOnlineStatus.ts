import { useEffect, useState } from "react";

export function useOnlineStatus(): boolean {
  const [isOnline, setIsOnline] = useState<boolean>(() =>
    typeof navigator !== "undefined" ? navigator.onLine : true,
  );

  useEffect(() => {
    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);

    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);

    // Chrome DevTools "Offline" throttling does not update navigator.onLine on initial reload.
    // Probe network to catch initial offline state on page load.
    fetch("/favicon.svg", { method: "HEAD", cache: "no-store" })
      .then((res) => {
        if (res.ok || res.type === "opaque") setIsOnline(true);
      })
      .catch(() => {
        setIsOnline(false);
      });

    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  }, []);

  return isOnline;
}
