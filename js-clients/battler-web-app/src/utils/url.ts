/**
 * Normalizes a user-provided or configured server URL into a valid WebSocket URL.
 * Automatically handles missing schemes and upgrades insecure ws:// to wss:// when running on HTTPS.
 */
export function normalizeWebSocketUrl(url: string, isHttps?: boolean): string {
  let target = url.trim();
  if (!target) return target;

  if (isHttps === undefined) {
    isHttps = typeof window !== "undefined" && window.location.protocol === "https:";
  }

  if (!target.startsWith("ws://") && !target.startsWith("wss://")) {
    const isLocal =
      target.startsWith("localhost") ||
      target.startsWith("127.0.0.1") ||
      target.startsWith("0.0.0.0");
    const scheme = isHttps || !isLocal ? "wss://" : "ws://";
    target = scheme + target;
  } else if (isHttps && target.startsWith("ws://")) {
    target = "wss://" + target.slice(5);
  }

  return target;
}
