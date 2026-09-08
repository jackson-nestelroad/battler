import { useEffect, useState } from "react";
import type {
  AbilityData,
  ConditionData,
  ItemData,
  MoveData,
} from "battler-data-service-client";
import { connectionManager } from "../core/wamp";

export type ResourceType = "move" | "ability" | "item" | "condition";

export interface ResourceMap {
  move: MoveData;
  ability: AbilityData;
  item: ItemData;
  condition: ConditionData;
}

const cache = new Map<string, unknown>();
const pending = new Map<string, Promise<unknown>>();

export function clearDataStoreCache(): void {
  cache.clear();
  pending.clear();
}

export async function fetchResource<T extends ResourceType>(
  type: T,
  query: string,
): Promise<ResourceMap[T] | null> {
  if (!query) return null;
  const key = `${type}:${query}`;
  if (cache.has(key)) return cache.get(key) as ResourceMap[T];
  if (pending.has(key)) return pending.get(key) as Promise<ResourceMap[T] | null>;

  const client = connectionManager.dataServiceClient;
  if (!client) return null;

  const promise = (async () => {
    try {
      let data: unknown = null;
      switch (type) {
        case "move":
          data = await client.getMove(query);
          break;
        case "ability":
          data = await client.getAbility(query);
          break;
        case "item":
          data = await client.getItem(query);
          break;
        case "condition":
          data = await client.getCondition(query);
          break;
      }
      if (data) cache.set(key, data);
      return data as ResourceMap[T];
    } catch {
      return null;
    } finally {
      pending.delete(key);
    }
  })();

  pending.set(key, promise);
  return promise;
}

export function useResourceData<T extends ResourceType>(
  type: T,
  query?: string | null,
): { data: ResourceMap[T] | null; loading: boolean } {
  const key = query ? `${type}:${query}` : "";
  const cached = key ? (cache.get(key) as ResourceMap[T] | undefined) ?? null : null;

  const [fetchedData, setFetchedData] = useState<{
    key: string;
    data: ResourceMap[T] | null;
  } | null>(null);
  const [loading, setLoading] = useState<boolean>(Boolean(key && !cached));

  const data = cached ?? (fetchedData?.key === key ? fetchedData.data : null);

  useEffect(() => {
    if (!key) {
      setLoading(false);
      return;
    }
    if (cache.has(key)) {
      setLoading(false);
      return;
    }

    let active = true;
    setLoading(true);

    fetchResource(type, query!).then((res) => {
      if (!active) return;
      setFetchedData({ key, data: res });
      setLoading(false);
    });

    return () => {
      active = false;
    };
  }, [type, key, query]);

  return { data, loading: Boolean(key && !data && loading) };
}

export const fetchMove = (nameOrId: string) => fetchResource("move", nameOrId);
export const fetchAbility = (nameOrId: string) => fetchResource("ability", nameOrId);
export const fetchItem = (nameOrId: string) => fetchResource("item", nameOrId);
export const fetchCondition = (nameOrId: string) => fetchResource("condition", nameOrId);

export const useMoveData = (nameOrId?: string | null) => useResourceData("move", nameOrId);
export const useAbilityData = (nameOrId?: string | null) => useResourceData("ability", nameOrId);
export const useItemData = (nameOrId?: string | null) => useResourceData("item", nameOrId);
export const useConditionData = (nameOrId?: string | null) => useResourceData("condition", nameOrId);
