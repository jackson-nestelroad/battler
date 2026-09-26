import { useEffect, useState } from "react";
import type {
  AbilitySummary,
  CatalogData,
  ItemSummary,
  MoveSummary,
  SpeciesSummary,
} from "battler-data-service-client";
import { DEFAULT_DEX_CATALOG } from "../data/defaultDexCatalog";
import { store } from "../store/store";

export type { AbilitySummary, CatalogData, ItemSummary, MoveSummary, SpeciesSummary };

let cachedCatalog: CatalogData = DEFAULT_DEX_CATALOG;
let pendingPromise: Promise<CatalogData> | null = null;

async function getDataServiceClient() {
  try {
    const { connectionManager } = await import("../core/wamp");
    return connectionManager.dataServiceClient;
  } catch {
    return null;
  }
}

export function getCachedCatalog(): CatalogData {
  return cachedCatalog;
}

export function setCachedCatalogForTesting(data: CatalogData): void {
  cachedCatalog = data;
  pendingPromise = null;
}

export function resetCachedCatalogForTesting(): void {
  cachedCatalog = DEFAULT_DEX_CATALOG;
  pendingPromise = null;
}

export async function fetchDexCatalog(): Promise<CatalogData> {
  const dataClient = await getDataServiceClient();
  if (!dataClient) {
    return cachedCatalog;
  }

  if (pendingPromise) return pendingPromise;

  const promise = dataClient
    .getCatalog()
    .then((data: CatalogData) => {
      cachedCatalog = {
        species: data.species?.length ? data.species : cachedCatalog.species,
        moves: data.moves?.length ? data.moves : cachedCatalog.moves,
        abilities: data.abilities?.length ? data.abilities : cachedCatalog.abilities,
        items: data.items?.length ? data.items : cachedCatalog.items,
      };
      pendingPromise = null;
      return cachedCatalog;
    })
    .catch((err: unknown) => {
      pendingPromise = null;
      console.warn("[useDexCatalog] Failed to fetch catalog from server, falling back to local catalog:", err);
      return cachedCatalog;
    });

  pendingPromise = promise;
  return promise;
}

export function useDexCatalog(): {
  catalog: CatalogData;
  loading: boolean;
  error: string | null;
} {
  const [catalog, setCatalog] = useState<CatalogData>(() => cachedCatalog);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    const tryFetch = () => {
      getDataServiceClient().then((client) => {
        if (!client || !active) return;
        setLoading(true);
        fetchDexCatalog()
          .then((res) => {
            if (!active) return;
            setCatalog(res);
            setLoading(false);
          })
          .catch((err) => {
            if (!active) return;
            setError(typeof err === "string" ? err : String(err));
            setLoading(false);
          });
      });
    };

    // 1. Try immediately in case client is already connected
    tryFetch();

    // 2. Subscribe to connection state changes in case connection establishes while mounted
    let lastStatus = store.getState()?.connection?.status;
    const unsubscribe = store.subscribe(() => {
      const currentStatus = store.getState()?.connection?.status;
      if (currentStatus === "connected" && lastStatus !== "connected") {
        tryFetch();
      }
      lastStatus = currentStatus;
    });

    return () => {
      active = false;
      unsubscribe();
    };
  }, []);

  return { catalog, loading, error };
}
