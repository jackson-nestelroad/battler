import { useEffect, useRef, useState } from "react";
import type {
  AbilityData,
  ConditionData,
  ItemData,
  MoveData,
  ResourceData,
  ResourceType,
  SpeciesData,
} from "battler-data-service-client";
import { connectionManager } from "../core/wamp";
import { toId } from "../utils/dataTooltipFormatting";

export type { ResourceData, ResourceType };
export { toId };

export interface ResourceMap {
  move: MoveData;
  ability: AbilityData;
  item: ItemData;
  condition: ConditionData;
  species: SpeciesData;
}

const cache = new Map<string, unknown>();
const pending = new Map<string, Promise<unknown>>();
const fxCached = new Set<string>();

function getResourceAliases(query: string, data?: unknown): string[] {
  const aliases = [query];
  const queryId = toId(query);
  if (queryId && queryId !== query) aliases.push(queryId);
  if (data && typeof data === "object" && "name" in data && typeof data.name === "string") {
    if (data.name !== query) aliases.push(data.name);
    const nameId = toId(data.name);
    if (nameId && nameId !== queryId && nameId !== query) aliases.push(nameId);
  }
  return aliases;
}

function markFxCached(type: ResourceType, query: string, data: unknown): void {
  for (const alias of getResourceAliases(query, data)) {
    fxCached.add(`${type}:${alias}`);
  }
}

function isFxCached(type: ResourceType, query: string): boolean {
  if (type === "species") return true;
  if (fxCached.has(`${type}:${query}`)) return true;
  const queryId = toId(query);
  if (queryId && fxCached.has(`${type}:${queryId}`)) return true;
  return false;
}

function cacheTypedResource(type: ResourceType, query: string, data: unknown): void {
  if (!data || typeof data !== "object") return;
  for (const alias of getResourceAliases(query, data)) {
    cache.set(`${type}:${alias}`, data);
  }
}

export function getCachedResource<T extends ResourceType>(
  type: T,
  query: string,
): ResourceMap[T] | undefined {
  if (!query) return undefined;
  const direct = cache.get(`${type}:${query}`);
  if (direct !== undefined) return direct as ResourceMap[T];
  const queryId = toId(query);
  if (queryId) {
    const idItem = cache.get(`${type}:${queryId}`);
    if (idItem !== undefined) return idItem as ResourceMap[T];
  }
  return undefined;
}

export function clearDataStoreCache(): void {
  cache.clear();
  pending.clear();
  fxCached.clear();
}

export async function fetchResource<T extends ResourceType>(
  type: T,
  query: string,
): Promise<ResourceMap[T] | null> {
  if (!query) return null;
  const cached = getCachedResource(type, query);
  if (cached !== undefined) return cached;

  const key = `${type}:${query}`;
  const queryId = toId(query);
  const idKey = queryId ? `${type}:${queryId}` : "";

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

const DEFAULT_RESOURCE_PRIORITY: readonly ResourceType[] = [
  "move",
  "ability",
  "item",
  "condition",
  "species",
];

function normalizeGenericResourceOptions(
  options?: GenericResourceLookupOptions | readonly ResourceType[] | ResourceType[],
): GenericResourceLookupOptions | undefined {
  if (!options) return undefined;
  if (Array.isArray(options)) {
    return { priority: options as readonly ResourceType[] };
  }
  return options as GenericResourceLookupOptions;
}

export function getGenericResourceCacheKey(
  query: string,
  options?: GenericResourceLookupOptions | readonly ResourceType[] | ResourceType[],
): string {
  if (!query) return "";
  const opts = normalizeGenericResourceOptions(options);
  const priorityKey =
    opts?.priority && opts.priority.length > 0 ? opts.priority.join(",") : "";
  return `resource:${query}:${priorityKey}`;
}

export function getCachedGenericResource(
  query: string,
  optionsOrPriority?: GenericResourceLookupOptions | readonly ResourceType[] | ResourceType[],
): ResourceData | undefined {
  if (!query) return undefined;
  const opts = normalizeGenericResourceOptions(optionsOrPriority);
  const key = getGenericResourceCacheKey(query, opts);
  const queryId = toId(query);
  const idKey = queryId ? getGenericResourceCacheKey(queryId, opts) : "";

  // 1. Direct key match (by raw query or normalized ID)
  for (const k of [key, idKey]) {
    if (!k) continue;
    const direct = cache.get(k) as ResourceData | undefined;
    if (direct !== undefined) {
      if (!opts?.include_fxlang || isFxCached(direct.type, query)) {
        return direct;
      }
    }
  }

  // 2. Typed cache fallback
  const searchTypes =
    opts?.priority && opts.priority.length > 0
      ? opts.priority
      : DEFAULT_RESOURCE_PRIORITY;
  for (const type of searchTypes) {
    const item = getCachedResource(type, query);
    if (item !== undefined) {
      if (opts?.include_fxlang && !isFxCached(type, query)) {
        continue;
      }
      const data = { type, data: item } as ResourceData;
      cache.set(key, data);
      if (idKey && idKey !== key) cache.set(idKey, data);
      return data;
    }
  }
  return undefined;
}

export async function fetchGenericResource(
  query: string,
  optionsOrPriority?: GenericResourceLookupOptions | readonly ResourceType[] | ResourceType[],
): Promise<ResourceData | null> {
  if (!query) return null;
  const options = normalizeGenericResourceOptions(optionsOrPriority);
  const key = getGenericResourceCacheKey(query, options);
  const cached = getCachedGenericResource(query, options);
  if (cached !== undefined) return cached;

  const queryId = toId(query);
  const idKey = queryId ? getGenericResourceCacheKey(queryId, options) : "";

  // Separate pending keys for fxlang so in-flight lightweight requests don't satisfy fxlang requests
  const fxSuffix = options?.include_fxlang ? ":fx" : "";
  const pendingKey = `${key}${fxSuffix}`;
  const pendingIdKey = idKey ? `${idKey}${fxSuffix}` : "";

  if (pending.has(pendingKey)) return pending.get(pendingKey) as Promise<ResourceData | null>;
  if (pendingIdKey && pending.has(pendingIdKey)) {
    return pending.get(pendingIdKey) as Promise<ResourceData | null>;
  }

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
        // Upgrade all relevant generic cache keys (both specified priority and default)
        const keysToCache = [key, getGenericResourceCacheKey(query)];
        if (queryId) {
          if (idKey) keysToCache.push(idKey);
          keysToCache.push(getGenericResourceCacheKey(queryId));
        }
        for (const k of keysToCache) {
          cache.set(k, data);
        }

        if (options?.include_fxlang) {
          markFxCached(data.type, query, data.data);
        }
        cacheTypedResource(data.type, query, data.data);
      }
      return data;
    } catch {
      return null;
    } finally {
      pending.delete(pendingKey);
      if (pendingIdKey) pending.delete(pendingIdKey);
    }
  })();

  pending.set(pendingKey, promise);
  if (pendingIdKey && pendingIdKey !== pendingKey) pending.set(pendingIdKey, promise);
  return promise;
}

function useAsyncCacheEntry<T>(
  key: string,
  getCached: () => T | undefined,
  fetcher: () => Promise<T | null>,
): { data: T | null; loading: boolean } {
  const cached = key ? getCached() ?? null : null;

  const [fetchedData, setFetchedData] = useState<{
    key: string;
    data: T | null;
  } | null>(null);

  const isFetched = fetchedData?.key === key;
  const data = cached ?? (isFetched ? fetchedData.data : null);
  const loading = Boolean(key && !cached && !isFetched);

  const getCachedRef = useRef(getCached);
  getCachedRef.current = getCached;
  const fetcherRef = useRef(fetcher);
  fetcherRef.current = fetcher;

  useEffect(() => {
    if (!key || getCachedRef.current() !== undefined) return;

    let active = true;
    fetcherRef.current().then((res) => {
      if (!active) return;
      setFetchedData({ key, data: res });
    });

    return () => {
      active = false;
    };
  }, [key]);

  return { data, loading };
}

export function useGenericResource(
  query?: string | null,
  optionsOrPriority?: GenericResourceLookupOptions | readonly ResourceType[] | ResourceType[],
): { data: ResourceData | null; loading: boolean } {
  const options = normalizeGenericResourceOptions(optionsOrPriority);
  const key = query ? getGenericResourceCacheKey(query, options) : "";

  return useAsyncCacheEntry(
    key,
    () => (query ? getCachedGenericResource(query, options) : undefined),
    () => (query ? fetchGenericResource(query, options) : Promise.resolve(null)),
  );
}

export function useResourceData<T extends ResourceType>(
  type: T,
  query?: string | null,
): { data: ResourceMap[T] | null; loading: boolean } {
  const key = query ? `${type}:${query}` : "";

  return useAsyncCacheEntry(
    key,
    () => (query ? getCachedResource(type, query) : undefined),
    () => (query ? fetchResource(type, query) : Promise.resolve(null)),
  );
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
