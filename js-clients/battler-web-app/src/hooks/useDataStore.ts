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

export interface LruResourceEntry {
  type: ResourceType;
  canonicalKey: string;
  aliases: Set<string>;
  data: unknown;
  genericWrapper?: ResourceData;
  description?: DescriptionData | null;
  hasFx: boolean;
}

export const DEFAULT_LRU_CAPACITY = 400;

export class ResourceLruStore {
  private capacity: number;
  private entries = new Map<string, LruResourceEntry>();
  private keyToCanonical = new Map<string, string>();

  constructor(capacity = DEFAULT_LRU_CAPACITY) {
    this.capacity = capacity;
  }

  public setCapacity(capacity: number): void {
    this.capacity = capacity;
    this.trimToCapacity();
  }

  public get(key: string): LruResourceEntry | undefined {
    if (!key) return undefined;
    const canonicalKey = this.keyToCanonical.get(key) || key;
    const entry = this.entries.get(canonicalKey);
    if (!entry) return undefined;

    // Refresh LRU position
    this.entries.delete(canonicalKey);
    this.entries.set(canonicalKey, entry);
    return entry;
  }

  public set(
    type: ResourceType,
    canonicalKey: string,
    aliases: Iterable<string>,
    data: unknown,
    description?: DescriptionData | null,
    isFx = false,
  ): void {
    if (!canonicalKey || !data || typeof data !== "object") return;

    const existing = this.entries.get(canonicalKey);
    if (!isFx && existing?.hasFx && type !== "species") {
      // Do not downgrade rich fxlang data to stripped data
      // But update aliases and description if provided
      if (description !== undefined && description !== null) {
        existing.description = description;
      }
      for (const alias of aliases) {
        existing.aliases.add(alias);
        this.keyToCanonical.set(alias, canonicalKey);
      }
      return;
    }

    if (existing) {
      this.entries.delete(canonicalKey);
    }

    const mergedAliases = new Set<string>(existing ? existing.aliases : []);
    for (const alias of aliases) {
      mergedAliases.add(alias);
    }

    const entry: LruResourceEntry = {
      type,
      canonicalKey,
      aliases: mergedAliases,
      data,
      description: description !== undefined ? description : existing?.description,
      hasFx: type === "species" || isFx || Boolean(existing?.hasFx),
    };

    this.trimToCapacity(this.capacity - 1);

    this.entries.set(canonicalKey, entry);
    for (const alias of mergedAliases) {
      this.keyToCanonical.set(alias, canonicalKey);
    }
  }

  public updateDescription(
    canonicalKey: string,
    description?: DescriptionData | null,
    extraAliases?: Iterable<string>,
  ): void {
    if (description === undefined) return;
    const entry = this.entries.get(canonicalKey);
    if (entry) {
      entry.description = description;
      if (extraAliases) {
        for (const alias of extraAliases) {
          entry.aliases.add(alias);
          this.keyToCanonical.set(alias, canonicalKey);
        }
      }
    }
  }

  public evict(canonicalKey: string): void {
    const entry = this.entries.get(canonicalKey);
    if (entry) {
      for (const alias of entry.aliases) {
        this.keyToCanonical.delete(alias);
      }
      this.entries.delete(canonicalKey);
    }
  }

  private trimToCapacity(max = this.capacity): void {
    while (this.entries.size > max && this.entries.size > 0) {
      const oldestKey = this.entries.keys().next().value;
      if (!oldestKey) break;
      this.evict(oldestKey);
    }
  }

  public clear(): void {
    this.entries.clear();
    this.keyToCanonical.clear();
  }

  public get size(): number {
    return this.entries.size;
  }
}

const lruStore = new ResourceLruStore();
const pending = new Map<string, Promise<unknown>>();

export function setLruCapacityForTesting(capacity: number): void {
  lruStore.setCapacity(capacity);
}

export function resetLruCapacityForTesting(): void {
  lruStore.setCapacity(DEFAULT_LRU_CAPACITY);
}

export function getLruSizeForTesting(): number {
  return lruStore.size;
}

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

function getCanonicalKey(type: ResourceType, query: string, data?: unknown): string {
  const name = extractResourceName(data);
  const id = (name ? toId(name) : "") || toId(query) || query.toLowerCase();
  return `${type}:${id}`;
}

function markFxCached(type: ResourceType, query: string, data: unknown): void {
  const canonicalKey = getCanonicalKey(type, query, data);
  const aliases = getResourceAliases(query, data).map((a) => `${type}:${a}`);
  lruStore.set(type, canonicalKey, aliases, data, undefined, true);
}

function isFxCached(type: ResourceType, query: string, name?: string): boolean {
  if (type === "species") return true;
  for (const q of [query, name]) {
    if (!q) continue;
    const entry = lruStore.get(`${type}:${q}`);
    if (entry?.hasFx) return true;
    const queryId = toId(q);
    if (queryId) {
      const idEntry = lruStore.get(`${type}:${queryId}`);
      if (idEntry?.hasFx) return true;
    }
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
  const canonicalKey = getCanonicalKey(type, query, data);
  const aliases: string[] = [];
  for (const alias of getResourceAliases(query, data)) {
    aliases.push(`${type}:${alias}`);
    const id = toId(alias);
    if (id) {
      aliases.push(`resource:${id}`);
    }
  }
  lruStore.set(type, canonicalKey, aliases, data, undefined, isFx);
}

function cacheTypedDescription(
  type: ResourceType,
  query: string,
  data: unknown,
  description?: DescriptionData | null,
): void {
  if (description === undefined) return;
  const canonicalKey = getCanonicalKey(type, query, data);
  const aliases = getResourceAliases(query, data).map((a) => `${type}:${a}`);
  lruStore.updateDescription(canonicalKey, description, aliases);
}

export function getCachedResource<T extends ResourceType>(
  type: T,
  query: string,
): ResourceMap[T] | undefined {
  if (!query) return undefined;
  const entry = lruStore.get(`${type}:${query}`);
  if (entry && entry.type === type) {
    return entry.data as ResourceMap[T];
  }
  const queryId = toId(query);
  if (queryId) {
    const idEntry = lruStore.get(`${type}:${queryId}`);
    if (idEntry && idEntry.type === type) {
      return idEntry.data as ResourceMap[T];
    }
  }
  return undefined;
}

export function getCachedDescription(
  type: ResourceType,
  query: string,
): DescriptionData | null | undefined {
  if (!query) return undefined;
  const entry = lruStore.get(`${type}:${query}`);
  if (entry && entry.type === type && entry.description !== undefined) {
    return entry.description;
  }
  const queryId = toId(query);
  if (queryId) {
    const idEntry = lruStore.get(`${type}:${queryId}`);
    if (idEntry && idEntry.type === type && idEntry.description !== undefined) {
      return idEntry.description;
    }
  }
  return undefined;
}

export function clearDataStoreCache(): void {
  lruStore.clear();
  pending.clear();
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
  const directEntry = lruStore.get(key);
  if (directEntry !== undefined) {
    if (!options?.include_fxlang || directEntry.hasFx || directEntry.type === "species") {
      if (!directEntry.genericWrapper) {
        directEntry.genericWrapper = {
          type: directEntry.type,
          data: directEntry.data,
        } as ResourceData;
      }
      return directEntry.genericWrapper;
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
      const canonicalKey = getCanonicalKey(type, query, item);
      const aliases = [key, `${type}:${toId(query)}`];
      const itemDesc = getCachedDescription(type, query);
      lruStore.set(type, canonicalKey, aliases, item, itemDesc);
      const entry = lruStore.get(canonicalKey);
      if (entry) {
        if (!entry.genericWrapper) {
          entry.genericWrapper = { type, data: item } as ResourceData;
        }
        return entry.genericWrapper;
      }
      return { type, data: item } as ResourceData;
    }
  }
  return undefined;
}

export function getCachedGenericDescription(
  query: string,
): DescriptionData | null | undefined {
  if (!query) return undefined;
  const key = getGenericResourceCacheKey(query);
  const entry = lruStore.get(key);
  if (entry && entry.description !== undefined) return entry.description;

  for (const type of DEFAULT_RESOURCE_SEARCH_ORDER) {
    const itemDesc = getCachedDescription(type, query);
    if (itemDesc !== undefined) {
      if (entry) {
        entry.description = itemDesc;
      }
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
          const canonicalKey = getCanonicalKey(data.type, query, data.data);
          const aliases = new Set<string>();
          aliases.add(key);
          if (dataName && dataName !== query) {
            aliases.add(getGenericResourceCacheKey(dataName));
          }
          for (const a of getResourceAliases(query, data.data)) {
            aliases.add(`${data.type}:${a}`);
            const id = toId(a);
            if (id) aliases.add(`resource:${id}`);
          }
          lruStore.set(
            data.type,
            canonicalKey,
            aliases,
            data.data,
            description,
            Boolean(options?.include_fxlang),
          );
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

/**
 * Fetches multiple species using the data service batch RPC endpoint.
 * Cached species are returned immediately, and uncached species are queried together
 * in a single network round-trip.
 */
export async function fetchSpeciesBatch(
  speciesList: string[],
): Promise<Record<string, SpeciesData | null>> {
  if (!speciesList || speciesList.length === 0) return {};

  const result: Record<string, SpeciesData | null> = {};
  const missing: string[] = [];

  for (const s of speciesList) {
    if (!s) continue;
    const cached = getCachedResource("species", s);
    if (cached !== undefined) {
      result[s] = cached;
    } else {
      missing.push(s);
    }
  }

  if (missing.length === 0) {
    return result;
  }

  const client = connectionManager.dataServiceClient;
  if (!client) {
    for (const m of missing) {
      result[m] = null;
    }
    return result;
  }

  try {
    const batchRes = await client.batch({
      moves: [],
      abilities: [],
      items: [],
      conditions: [],
      species: missing,
      options: { include_fxlang: false },
    });
    if (batchRes && batchRes.species) {
      for (const [key, data] of Object.entries(batchRes.species)) {
        if (data) {
          cacheTypedResource("species", key, data);
          const desc = batchRes.descriptions?.[key] ?? null;
          cacheTypedDescription("species", key, data, desc);
          result[key] = data;
        } else {
          result[key] = null;
        }
      }
    }
  } catch {
    for (const m of missing) {
      result[m] = null;
    }
  }

  return result;
}

export const useMoveData = (nameOrId?: string | null) => useResourceData("move", nameOrId);
export const useAbilityData = (nameOrId?: string | null) => useResourceData("ability", nameOrId);
export const useItemData = (nameOrId?: string | null) => useResourceData("item", nameOrId);
export const useConditionData = (nameOrId?: string | null) => useResourceData("condition", nameOrId);
export const useSpeciesData = (nameOrId?: string | null) => useResourceData("species", nameOrId);


