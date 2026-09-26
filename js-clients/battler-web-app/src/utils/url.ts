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

/**
 * Checks whether a boolean search parameter/flag is present and active in a query string or current URL.
 * Supports flags without values (e.g. `?flag`), or values other than "false" and "0".
 * Also checks hash queries (e.g. `#/path?flag`) as fallback when running in a browser environment.
 */
export function hasBooleanSearchParam(param: string, search?: string): boolean {
  if (typeof window === "undefined" && search === undefined) {
    return false;
  }
  const query = search !== undefined ? search : window.location.search;
  const params = new URLSearchParams(query);
  if (params.has(param)) {
    const val = params.get(param);
    return val !== "false" && val !== "0";
  }
  if (
    search === undefined &&
    typeof window !== "undefined" &&
    window.location.hash.includes("?")
  ) {
    const hashQuery = window.location.hash.slice(window.location.hash.indexOf("?"));
    const hashParams = new URLSearchParams(hashQuery);
    if (hashParams.has(param)) {
      const val = hashParams.get(param);
      return val !== "false" && val !== "0";
    }
  }
  return false;
}
