import { useEffect, useRef, useState } from "react";
import type {
  AbilityData,
  ConditionData,
  DescriptionData,
  ItemData,
  MoveData,
  ResourceData,
  ResourceType,
  SpeciesData,
} from "battler-data-service-client";
import { connectionManager } from "../core/wamp";
import { extractResourceName, toId } from "../utils/dataTooltipFormatting";

export type { DescriptionData, ResourceData, ResourceType };
export { toId };

export interface ResourceMap {
  move: MoveData;
  ability: AbilityData;
  item: ItemData;
  condition: ConditionData;
  species: SpeciesData;
}

const cache = new Map<string, unknown>();
const descriptionCache = new Map<string, DescriptionData | null>();
const pending = new Map<string, Promise<unknown>>();
const fxCached = new Set<string>();

function getResourceAliases(query: string, data?: unknown): string[] {
  const aliases = new Set<string>();
  if (query) {
    aliases.add(query);
    const queryId = toId(query);
    if (queryId) aliases.add(queryId);
  }
  const name = extractResourceName(data);
  if (name) {
    aliases.add(name);
    const nameId = toId(name);
    if (nameId) aliases.add(nameId);
  }
  return Array.from(aliases);
}

function markFxCached(type: ResourceType, query: string, data: unknown): void {
  for (const alias of getResourceAliases(query, data)) {
    fxCached.add(`${type}:${alias}`);
  }
}

function isFxCached(type: ResourceType, query: string, name?: string): boolean {
  if (type === "species") return true;
  for (const q of [query, name]) {
    if (!q) continue;
    if (fxCached.has(`${type}:${q}`)) return true;
    const queryId = toId(q);
    if (queryId && fxCached.has(`${type}:${queryId}`)) return true;
  }
  return false;
}

function hasFxAstCached(type: ResourceType, query: string, name?: string): boolean {
  if (type === "species") return false;
  return isFxCached(type, query, name);
}

function cacheTypedResource(
  type: ResourceType,
  query: string,
  data: unknown,
  isFx = false,
): void {
  if (!data || typeof data !== "object") return;
  const name = extractResourceName(data);
  if (!isFx && hasFxAstCached(type, query, name)) {
    // Do not downgrade rich fxlang data to stripped data
    return;
  }
  for (const alias of getResourceAliases(query, data)) {
    cache.set(`${type}:${alias}`, data);
  }
}

function cacheTypedDescription(
  type: ResourceType,
  query: string,
  data: unknown,
  description?: DescriptionData | null,
): void {
  if (description === undefined) return;
  for (const alias of getResourceAliases(query, data)) {
    descriptionCache.set(`${type}:${alias}`, description);
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

export function getCachedDescription(
  type: ResourceType,
  query: string,
): DescriptionData | null | undefined {
  if (!query) return undefined;
  const direct = descriptionCache.get(`${type}:${query}`);
  if (direct !== undefined) return direct;
  const queryId = toId(query);
  if (queryId) {
    const idItem = descriptionCache.get(`${type}:${queryId}`);
    if (idItem !== undefined) return idItem;
  }
  return undefined;
}

export function clearDataStoreCache(): void {
  cache.clear();
  descriptionCache.clear();
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
      let res: unknown = null;
      switch (type) {
        case "move":
          res = await client.getMove(query);
          break;
        case "ability":
          res = await client.getAbility(query);
          break;
        case "item":
          res = await client.getItem(query);
          break;
        case "condition":
          res = await client.getCondition(query);
          break;
        case "species":
          res = await client.getSpecies(query);
          break;
      }
      if (!res) return null;

      let data: unknown = res;
      let description: DescriptionData | null = null;
      if (typeof res === "object" && res !== null && "data" in res) {
        data = (res as any).data;
        description = (res as any).description ?? null;
      }

      if (data) {
        cacheTypedResource(type, query, data);
        cacheTypedDescription(type, query, data, description);
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
  include_fxlang?: boolean;
};

const DEFAULT_RESOURCE_SEARCH_ORDER: readonly ResourceType[] = [
  "condition",
  "move",
  "ability",
  "item",
  "species",
];

export function getGenericResourceCacheKey(query: string): string {
  if (!query) return "";
  const id = toId(query) || query.toLowerCase();
  return `resource:${id}`;
}

export function getCachedGenericResource(
  query: string,
  options?: GenericResourceLookupOptions,
): ResourceData | undefined {
  if (!query) return undefined;
  const key = getGenericResourceCacheKey(query);

  // 1. Direct key match (by normalized ID)
  const direct = cache.get(key) as ResourceData | undefined;
  if (direct !== undefined) {
    const dataName = extractResourceName(direct.data);
    if (!options?.include_fxlang || isFxCached(direct.type, query, dataName)) {
      return direct;
    }
  }

  // 2. Typed cache fallback in default search order
  for (const type of DEFAULT_RESOURCE_SEARCH_ORDER) {
    const item = getCachedResource(type, query);
    if (item !== undefined) {
      const itemName = extractResourceName(item);
      if (options?.include_fxlang && !isFxCached(type, query, itemName)) {
        continue;
      }
      const data = { type, data: item } as ResourceData;
      cache.set(key, data);
      const itemDesc = getCachedDescription(type, query);
      if (itemDesc !== undefined) {
        descriptionCache.set(key, itemDesc);
      }
      return data;
    }
  }
  return undefined;
}

export function getCachedGenericDescription(
  query: string,
): DescriptionData | null | undefined {
  if (!query) return undefined;
  const key = getGenericResourceCacheKey(query);
  const direct = descriptionCache.get(key);
  if (direct !== undefined) return direct;

  for (const type of DEFAULT_RESOURCE_SEARCH_ORDER) {
    const itemDesc = getCachedDescription(type, query);
    if (itemDesc !== undefined) {
      descriptionCache.set(key, itemDesc);
      return itemDesc;
    }
  }
  return undefined;
}

export async function fetchGenericResource(
  query: string,
  options?: GenericResourceLookupOptions,
): Promise<ResourceData | null> {
  if (!query) return null;
  const key = getGenericResourceCacheKey(query);
  const cached = getCachedGenericResource(query, options);
  if (cached !== undefined) return cached;

  // Separate pending keys for fxlang so in-flight lightweight requests don't satisfy fxlang requests
  const pendingKey = `${key}${options?.include_fxlang ? ":fx" : ""}`;
  if (pending.has(pendingKey)) return pending.get(pendingKey) as Promise<ResourceData | null>;

  // If a request WITH fxlang is already in flight, it satisfies lightweight requests too
  if (!options?.include_fxlang && pending.has(`${key}:fx`)) {
    return pending.get(`${key}:fx`) as Promise<ResourceData | null>;
  }

  const client = connectionManager.dataServiceClient;
  if (!client) return null;

  const promise = (async () => {
    try {
      const res = await client.getResource(
        query,
        options ? { include_fxlang: options.include_fxlang } : undefined,
      );
      if (!res) return null;

      let data: ResourceData | null = null;
      let description: DescriptionData | null = null;
      if ("data" in res && (res as any).data && "type" in (res as any).data) {
        data = (res as any).data;
        description = (res as any).description ?? null;
      } else if ("type" in res && "data" in res) {
        data = res as unknown as ResourceData;
        description = (res as any).description ?? null;
      }

      if (data) {
        const dataName = extractResourceName(data.data);
        const alreadyFx = hasFxAstCached(data.type, query, dataName);
        if (!alreadyFx || options?.include_fxlang) {
          cache.set(key, data);
          if (description !== undefined) {
            descriptionCache.set(key, description);
          }
          if (dataName && dataName !== query) {
            const aliasKey = getGenericResourceCacheKey(dataName);
            cache.set(aliasKey, data);
            if (description !== undefined) {
              descriptionCache.set(aliasKey, description);
            }
          }
          cacheTypedResource(data.type, query, data.data, Boolean(options?.include_fxlang));
          cacheTypedDescription(data.type, query, data.data, description);
        }

        if (options?.include_fxlang) {
          markFxCached(data.type, query, data.data);
        }
      }
      return data;
    } catch {
      return null;
    } finally {
      pending.delete(pendingKey);
    }
  })();

  pending.set(pendingKey, promise);
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
    fetcherRef.current()
      .then((res) => {
        if (!active) return;
        setFetchedData({ key, data: res });
      })
      .catch(() => {
        if (!active) return;
        setFetchedData({ key, data: null });
      });

    return () => {
      active = false;
    };
  }, [key]);

  return { data, loading };
}

export function useGenericResource(
  query?: string | null,
  options?: GenericResourceLookupOptions,
): { data: ResourceData | null; description?: DescriptionData | null; loading: boolean } {
  const key = query ? getGenericResourceCacheKey(query) : "";
  const hookKey = query ? `${key}${options?.include_fxlang ? ":fx" : ""}` : "";

  const { data, loading } = useAsyncCacheEntry(
    hookKey,
    () => (query ? getCachedGenericResource(query, options) : undefined),
    () => (query ? fetchGenericResource(query, options) : Promise.resolve(null)),
  );

  const description = query ? getCachedGenericDescription(query) ?? null : null;

  return { data, description, loading };
}

export function useResourceData<T extends ResourceType>(
  type: T,
  query?: string | null,
): { data: ResourceMap[T] | null; description?: DescriptionData | null; loading: boolean } {
  const key = query ? `${type}:${query}` : "";

  const { data, loading } = useAsyncCacheEntry(
    key,
    () => (query ? getCachedResource(type, query) : undefined),
    () => (query ? fetchResource(type, query) : Promise.resolve(null)),
  );

  const description = query ? getCachedDescription(type, query) ?? null : null;

  return { data, description, loading };
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

