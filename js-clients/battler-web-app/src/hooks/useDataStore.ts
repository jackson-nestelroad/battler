import { useEffect, useRef, useState } from "react";
import type {
  AbilityData,
  ConditionData,
  ItemData,
  MoveData,
  ResourceData,
  SpeciesData,
} from "battler-data-service-client";
import { connectionManager } from "../core/wamp";

export type ResourceType = "move" | "ability" | "item" | "condition" | "species";

export interface ResourceMap {
  move: MoveData;
  ability: AbilityData;
  item: ItemData;
  condition: ConditionData;
  species: SpeciesData;
}

const cache = new Map<string, unknown>();
const pending = new Map<string, Promise<unknown>>();

export const toId = (str: string) => str.toLowerCase().replace(/[^a-z0-9]/g, "");

function cacheTypedResource(type: ResourceType, query: string, data: unknown): void {
  if (!data || typeof data !== "object") return;
  const name = "name" in data && typeof data.name === "string" ? data.name : undefined;

  // 1. Raw query string
  cache.set(`${type}:${query}`, data);

  // 2. Normalized query ID
  const queryId = toId(query);
  if (queryId) {
    cache.set(`${type}:${queryId}`, data);
  }

  // 3. Canonical name and normalized name ID
  if (name) {
    cache.set(`${type}:${name}`, data);
    const nameId = toId(name);
    if (nameId) {
      cache.set(`${type}:${nameId}`, data);
    }
  }
}

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
  const queryId = toId(query);
  const idKey = queryId ? `${type}:${queryId}` : "";
  if (idKey && cache.has(idKey)) return cache.get(idKey) as ResourceMap[T];

  if (pending.has(key)) return pending.get(key) as Promise<ResourceMap[T] | null>;
  if (idKey && pending.has(idKey)) return pending.get(idKey) as Promise<ResourceMap[T] | null>;

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
        case "species":
          data = await client.getSpecies(query);
          break;
      }
      if (data) {
        cacheTypedResource(type, query, data);
      }
      return data as ResourceMap[T];
    } catch {
      return null;
    } finally {
      pending.delete(key);
      if (idKey) pending.delete(idKey);
    }
  })();

  pending.set(key, promise);
  if (idKey && idKey !== key) pending.set(idKey, promise);
  return promise;
}

export type GenericResourceLookupOptions = {
  priority?: readonly ResourceType[] | ResourceType[];
  include_fxlang?: boolean;
};

export async function fetchGenericResource(
  query: string,
  options?: GenericResourceLookupOptions,
): Promise<ResourceData | null> {
  if (!query) return null;
  const priorityKey = options?.priority ? JSON.stringify(options.priority) : "";
  const key = `resource:${query}:${priorityKey}`;
  if (cache.has(key)) return cache.get(key) as ResourceData;

  if (options?.priority?.length === 1) {
    const singleType = options.priority[0];
    const queryId = toId(query);
    const singleKey = `${singleType}:${query}`;
    const singleIdKey = queryId ? `${singleType}:${queryId}` : "";
    const cachedItem = (cache.get(singleKey) ?? (singleIdKey ? cache.get(singleIdKey) : undefined)) as unknown;
    if (cachedItem) {
      const result = { type: singleType, data: cachedItem } as ResourceData;
      cache.set(key, result);
      return result;
    }
  }

  if (pending.has(key)) return pending.get(key) as Promise<ResourceData | null>;

  const client = connectionManager.dataServiceClient;
  if (!client) return null;

  const promise = (async () => {
    try {
      const data = await client.getResource(
        query,
        options
          ? {
              priority: options.priority ? [...options.priority] : undefined,
              include_fxlang: options.include_fxlang,
            }
          : undefined,
      );
      if (data) {
        cache.set(key, data);
        cacheTypedResource(data.type, query, data.data);
      }
      return data;
    } catch {
      return null;
    } finally {
      pending.delete(key);
    }
  })();

  pending.set(key, promise);
  return promise;
}

export function useGenericResource(
  query?: string | null,
  options?: GenericResourceLookupOptions,
): { data: ResourceData | null; loading: boolean } {
  const optionsKey = options ? JSON.stringify(options) : "";
  const optionsRef = useRef(options);
  optionsRef.current = options;

  const key = query ? `resource:${query}:${optionsKey}` : "";
  let cached = key ? (cache.get(key) as ResourceData | undefined) ?? null : null;

  if (!cached && query && options?.priority?.length === 1) {
    const singleType = options.priority[0];
    const queryId = toId(query);
    const singleKey = `${singleType}:${query}`;
    const singleIdKey = queryId ? `${singleType}:${queryId}` : "";
    const cachedItem = (cache.get(singleKey) ?? (singleIdKey ? cache.get(singleIdKey) : undefined)) as unknown;
    if (cachedItem) {
      cached = { type: singleType, data: cachedItem } as ResourceData;
    }
  }

  const [fetchedData, setFetchedData] = useState<{
    key: string;
    data: ResourceData | null;
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

    fetchGenericResource(query!, optionsRef.current).then((res) => {
      if (!active) return;
      setFetchedData({ key, data: res });
      setLoading(false);
    });

    return () => {
      active = false;
    };
  }, [key, query, optionsKey]);

  return { data, loading: Boolean(key && !data && loading) };
}

export function useResourceData<T extends ResourceType>(
  type: T,
  query?: string | null,
): { data: ResourceMap[T] | null; loading: boolean } {
  const queryId = query ? toId(query) : "";
  const key = query ? `${type}:${query}` : "";
  const idKey = queryId ? `${type}:${queryId}` : "";

  const cached = key
    ? ((cache.get(key) ?? (idKey ? cache.get(idKey) : undefined)) as
        | ResourceMap[T]
        | undefined) ?? null
    : null;

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
    if (cache.has(key) || (idKey && cache.has(idKey))) {
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
  }, [type, key, idKey, query]);

  return { data, loading: Boolean(key && !data && loading) };
}

export const fetchMove = (nameOrId: string) => fetchResource("move", nameOrId);
export const fetchAbility = (nameOrId: string) => fetchResource("ability", nameOrId);
export const fetchItem = (nameOrId: string) => fetchResource("item", nameOrId);
export const fetchCondition = (nameOrId: string) => fetchResource("condition", nameOrId);
export const fetchSpecies = (nameOrId: string) => fetchResource("species", nameOrId);

export const useMoveData = (nameOrId?: string | null) => useResourceData("move", nameOrId);
export const useAbilityData = (nameOrId?: string | null) => useResourceData("ability", nameOrId);
export const useItemData = (nameOrId?: string | null) => useResourceData("item", nameOrId);
export const useConditionData = (nameOrId?: string | null) => useResourceData("condition", nameOrId);
export const useSpeciesData = (nameOrId?: string | null) => useResourceData("species", nameOrId);
