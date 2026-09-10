import type {
  AbilityData,
  ConditionData,
  ItemData,
  MoveData,
} from "battler-data-service-client";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as wampModule from "../core/wamp";
import {
  clearDataStoreCache,
  fetchAbility,
  fetchCondition,
  fetchGenericResource,
  fetchItem,
  fetchMove,
  fetchSpecies,
  getCachedGenericResource,
  getGenericResourceCacheKey,
} from "./useDataStore";

describe("useDataStore", () => {
  const mockMove = {
    name: "Thunderbolt",
    category: "Special",
    primary_type: "Electric",
    base_power: 90,
    accuracy: 100,
    pp: 15,
    priority: 0,
    target: "Normal",
    flags: ["Contact"],
  } as unknown as MoveData;

  const mockAbility = {
    name: "Intimidate",
    flags: ["Breakable"],
  } as unknown as AbilityData;

  const mockItem = {
    name: "Leftovers",
    flags: [],
  } as unknown as ItemData;

  const mockCondition = {
    name: "Rain",
    condition_type: "Weather",
  } as unknown as ConditionData;

  const mockClient = {
    getMove: vi.fn(),
    getAbility: vi.fn(),
    getItem: vi.fn(),
    getCondition: vi.fn(),
    getSpecies: vi.fn(),
    getResource: vi.fn(),
  };

  beforeEach(() => {
    clearDataStoreCache();
    vi.clearAllMocks();
    wampModule.connectionManager.dataServiceClient = mockClient as any;
  });

  describe("fetchResource & caching", () => {
    it("fetches and caches move data", async () => {
      mockClient.getMove.mockResolvedValueOnce(mockMove);

      const res1 = await fetchMove("Thunderbolt");
      expect(res1).toEqual(mockMove);
      expect(mockClient.getMove).toHaveBeenCalledTimes(1);
      expect(mockClient.getMove).toHaveBeenCalledWith("Thunderbolt");

      // Subsequent call should hit cache immediately
      const res2 = await fetchMove("Thunderbolt");
      expect(res2).toEqual(mockMove);
      expect(mockClient.getMove).toHaveBeenCalledTimes(1);
    });

    it("deduplicates simultaneous in-flight requests", async () => {
      let resolvePromise: (data: MoveData) => void;
      mockClient.getMove.mockReturnValueOnce(
        new Promise((resolve) => {
          resolvePromise = resolve;
        }),
      );

      const p1 = fetchMove("Thunderbolt");
      const p2 = fetchMove("Thunderbolt");

      expect(mockClient.getMove).toHaveBeenCalledTimes(1);

      resolvePromise!(mockMove);
      const [r1, r2] = await Promise.all([p1, p2]);

      expect(r1).toEqual(mockMove);
      expect(r2).toEqual(mockMove);
      expect(mockClient.getMove).toHaveBeenCalledTimes(1);
    });

    it("handles null/empty query gracefully", async () => {
      const res = await fetchMove("");
      expect(res).toBeNull();
      expect(mockClient.getMove).not.toHaveBeenCalled();
    });

    it("handles client errors gracefully", async () => {
      mockClient.getMove.mockRejectedValueOnce(new Error("RPC failed"));

      const res = await fetchMove("unknown_move");
      expect(res).toBeNull();
    });
  });

  describe("fetchAbility, fetchItem, fetchCondition", () => {
    it("fetches and caches ability", async () => {
      mockClient.getAbility.mockResolvedValueOnce(mockAbility);
      const res = await fetchAbility("Intimidate");
      expect(res).toEqual(mockAbility);

      const cached = await fetchAbility("Intimidate");
      expect(cached).toEqual(mockAbility);
      expect(mockClient.getAbility).toHaveBeenCalledTimes(1);
    });

    it("fetches and caches item", async () => {
      mockClient.getItem.mockResolvedValueOnce(mockItem);
      const res = await fetchItem("Leftovers");
      expect(res).toEqual(mockItem);

      const cached = await fetchItem("Leftovers");
      expect(cached).toEqual(mockItem);
      expect(mockClient.getItem).toHaveBeenCalledTimes(1);
    });

    it("fetches and caches condition", async () => {
      mockClient.getCondition.mockResolvedValueOnce(mockCondition);
      const res = await fetchCondition("Rain");
      expect(res).toEqual(mockCondition);

      const cached = await fetchCondition("Rain");
      expect(cached).toEqual(mockCondition);
      expect(mockClient.getCondition).toHaveBeenCalledTimes(1);
    });

    it("fetches and caches species", async () => {
      const mockSpecies = {
        name: "Pikachu",
        primary_type: "Electric",
      };
      mockClient.getSpecies.mockResolvedValueOnce(mockSpecies);
      const res = await fetchSpecies("Pikachu");
      expect(res).toEqual(mockSpecies);

      const cached = await fetchSpecies("Pikachu");
      expect(cached).toEqual(mockSpecies);
      expect(mockClient.getSpecies).toHaveBeenCalledTimes(1);
    });
  });

  describe("fetchGenericResource", () => {
    it("fetches and caches generic resource", async () => {
      const mockResolved = {
        type: "move" as const,
        data: mockMove,
      };
      mockClient.getResource.mockResolvedValueOnce(mockResolved);

      const res = await fetchGenericResource("Toxic Spikes", {
        priority: ["condition", "move", "ability", "item"],
      });
      expect(res).toEqual(mockResolved);
      expect(mockClient.getResource).toHaveBeenCalledTimes(1);

      // Cached call
      const cached = await fetchGenericResource("Toxic Spikes", {
        priority: ["condition", "move", "ability", "item"],
      });
      expect(cached).toEqual(mockResolved);
      expect(mockClient.getResource).toHaveBeenCalledTimes(1);
    });

    it("cross-populates typed cache when generic resource resolves", async () => {
      const mockResolved = {
        type: "move" as const,
        data: mockMove,
      };
      mockClient.getResource.mockResolvedValueOnce(mockResolved);

      await fetchGenericResource("Reflect", {
        priority: ["condition", "move", "ability", "item"],
      });

      // Now fetchMove should hit cache without calling getMove
      const move1 = await fetchMove("Reflect");
      expect(move1).toEqual(mockMove);
      expect(mockClient.getMove).not.toHaveBeenCalled();

      // Canonical name and normalized toId should also hit cache
      const move2 = await fetchMove("reflect");
      expect(move2).toEqual(mockMove);
      expect(mockClient.getMove).not.toHaveBeenCalled();
    });

    it("uses typed cache for generic resource lookup when priority is a single type", async () => {
      mockClient.getMove.mockResolvedValueOnce(mockMove);
      await fetchMove("Thunderbolt");
      expect(mockClient.getMove).toHaveBeenCalledTimes(1);

      const res = await fetchGenericResource("thunderbolt", {
        priority: ["move"],
      });

      expect(res).toEqual({ type: "move", data: mockMove });
      expect(mockClient.getResource).not.toHaveBeenCalled();
    });

    it("uses typed cache for multi-priority generic resource lookup without calling getResource", async () => {
      mockClient.getMove.mockResolvedValueOnce(mockMove);
      await fetchMove("Thunderbolt");
      expect(mockClient.getMove).toHaveBeenCalledTimes(1);

      const res = await fetchGenericResource("Thunderbolt", {
        priority: ["condition", "move", "ability", "item"],
      });

      expect(res).toEqual({ type: "move", data: mockMove });
      expect(mockClient.getResource).not.toHaveBeenCalled();
    });

    it("respects priority order in cached multi-priority lookups", async () => {
      // Suppose an item and an ability share the same query name
      mockClient.getAbility.mockResolvedValueOnce(mockAbility);
      mockClient.getItem.mockResolvedValueOnce(mockItem);
      await fetchAbility("Shared");
      await fetchItem("Shared");

      const resItemFirst = await fetchGenericResource("Shared", {
        priority: ["item", "ability"],
      });
      expect(resItemFirst?.type).toBe("item");

      const resAbilityFirst = await fetchGenericResource("Shared", {
        priority: ["ability", "item"],
      });
      expect(resAbilityFirst?.type).toBe("ability");
    });

    it("returns stable object references across multiple getCachedGenericResource calls", async () => {
      mockClient.getMove.mockResolvedValueOnce(mockMove);
      await fetchMove("Thunderbolt");

      const ref1 = getCachedGenericResource("Thunderbolt");
      const ref2 = getCachedGenericResource("Thunderbolt");

      expect(ref1).toBeDefined();
      expect(ref1).toBe(ref2);
    });

    it("populates normalized ID cache key on getCachedGenericResource for instant lookup", async () => {
      mockClient.getMove.mockResolvedValueOnce(mockMove);
      await fetchMove("Thunder Wave");

      // First query with original name
      const resName = getCachedGenericResource("Thunder Wave");
      expect(resName?.data.name).toBe("Thunderbolt");

      // Subsequent query with normalized ID directly hits cache without calling getResource
      const resId = getCachedGenericResource("thunderwave");
      expect(resId?.data.name).toBe("Thunderbolt");
      expect(mockClient.getResource).not.toHaveBeenCalled();
    });

    it("deduplicates simultaneous in-flight requests across query name and ID", async () => {
      let resolvePromise: (data: any) => void;
      mockClient.getResource.mockReturnValueOnce(
        new Promise((resolve) => {
          resolvePromise = resolve;
        }),
      );

      const p1 = fetchGenericResource("Thunder Wave");
      const p2 = fetchGenericResource("thunderwave");

      expect(mockClient.getResource).toHaveBeenCalledTimes(1);

      const mockWave = {
        type: "move" as const,
        data: { ...mockMove, name: "Thunder Wave" },
      };
      resolvePromise!(mockWave);

      const [r1, r2] = await Promise.all([p1, p2]);
      expect(r1).toEqual(mockWave);
      expect(r2).toEqual(mockWave);
      expect(mockClient.getResource).toHaveBeenCalledTimes(1);
    });

    it("accepts an array of ResourceType as priority shorthand", async () => {
      const mockResolved = {
        type: "item" as const,
        data: mockItem,
      };
      mockClient.getResource.mockResolvedValueOnce(mockResolved);

      const res = await fetchGenericResource("Leftovers", ["item", "ability"]);
      expect(res).toEqual(mockResolved);
      expect(mockClient.getResource).toHaveBeenCalledWith("Leftovers", {
        priority: ["item", "ability"],
        include_fxlang: undefined,
      });
    });

    it("bypasses typed cache when include_fxlang is true", async () => {
      mockClient.getMove.mockResolvedValueOnce(mockMove);
      await fetchMove("Thunderbolt");
      expect(mockClient.getMove).toHaveBeenCalledTimes(1);

      // getCachedGenericResource should return undefined when include_fxlang is requested
      const cached = getCachedGenericResource("Thunderbolt", {
        include_fxlang: true,
      });
      expect(cached).toBeUndefined();

      // fetchGenericResource should call getResource with include_fxlang
      const fxResolved = {
        type: "move" as const,
        data: { ...mockMove, fxlang: "effects" },
      };
      mockClient.getResource.mockResolvedValueOnce(fxResolved);

      const res = await fetchGenericResource("Thunderbolt", {
        include_fxlang: true,
      });
      expect(res).toEqual(fxResolved);
      expect(mockClient.getResource).toHaveBeenCalledWith("Thunderbolt", {
        priority: undefined,
        include_fxlang: true,
      });
    });

    it("upgrades cached entry when looking up fxlang and does not re-fetch later", async () => {
      // 1. Initial lookup without fxlang (e.g. tooltip hover)
      mockClient.getResource.mockResolvedValueOnce({
        type: "move" as const,
        data: mockMove,
      });
      const initial = await fetchGenericResource("Thunderbolt");
      expect(initial?.data).toEqual(mockMove);
      expect(mockClient.getResource).toHaveBeenCalledTimes(1);

      // 2. Later looking up with fxlang (e.g. FxLangModal opening)
      const fxData = {
        ...mockMove,
        effect: { ast: "test_ast" },
      };
      mockClient.getResource.mockResolvedValueOnce({
        type: "move" as const,
        data: fxData,
      });
      const withFx = await fetchGenericResource("Thunderbolt", {
        priority: ["condition", "move", "ability", "item"],
        include_fxlang: true,
      });
      expect(withFx?.data).toEqual(fxData);
      expect(mockClient.getResource).toHaveBeenCalledTimes(2);

      // 3. Subsequent fxlang lookup with the same options hits cache immediately
      const fxAgain = await fetchGenericResource("Thunderbolt", {
        priority: ["condition", "move", "ability", "item"],
        include_fxlang: true,
      });
      expect(fxAgain?.data).toEqual(fxData);
      expect(mockClient.getResource).toHaveBeenCalledTimes(2); // No new network call!

      // 4. Subsequent fxlang lookup with different priority also hits cache!
      const fxDifferentPriority = await fetchGenericResource("Thunderbolt", {
        priority: ["move"],
        include_fxlang: true,
      });
      expect(fxDifferentPriority?.data).toEqual(fxData);
      expect(mockClient.getResource).toHaveBeenCalledTimes(2); // No new network call!

      // 5. Subsequent non-fx lookup also gets the enriched data from cache
      const nonFxAgain = await fetchGenericResource("Thunderbolt");
      expect(nonFxAgain?.data).toEqual(fxData);
      expect(mockClient.getResource).toHaveBeenCalledTimes(2); // No new network call!
    });

    it("satisfies fxlang lookups immediately for species from cache", async () => {
      const mockSpecies = { name: "Pikachu", primary_type: "Electric" };
      mockClient.getSpecies.mockResolvedValueOnce(mockSpecies);
      await fetchSpecies("Pikachu");
      expect(mockClient.getSpecies).toHaveBeenCalledTimes(1);

      // Species has no fxlang AST bytecode, so cached species satisfies include_fxlang
      const res = await fetchGenericResource("Pikachu", {
        include_fxlang: true,
      });
      expect(res).toEqual({ type: "species", data: mockSpecies });
      expect(mockClient.getResource).not.toHaveBeenCalled();
    });
  });

  describe("clearDataStoreCache", () => {
    it("clears cached entries", async () => {
      mockClient.getMove.mockResolvedValue(mockMove);
      await fetchMove("Thunderbolt");
      expect(mockClient.getMove).toHaveBeenCalledTimes(1);

      clearDataStoreCache();

      await fetchMove("Thunderbolt");
      expect(mockClient.getMove).toHaveBeenCalledTimes(2);
    });
  });

  describe("getGenericResourceCacheKey", () => {
    it("builds consistent keys for empty, single, and multi-priority options", () => {
      expect(getGenericResourceCacheKey("")).toBe("");
      expect(getGenericResourceCacheKey("Toxic Spikes")).toBe("resource:Toxic Spikes:");
      expect(getGenericResourceCacheKey("Toxic Spikes", { priority: ["condition", "move"] })).toBe(
        "resource:Toxic Spikes:condition,move",
      );
      expect(getGenericResourceCacheKey("Toxic Spikes", ["condition", "move"])).toBe(
        "resource:Toxic Spikes:condition,move",
      );
    });

    it("uses unified keys regardless of include_fxlang", () => {
      expect(
        getGenericResourceCacheKey("Rain", {
          priority: ["condition"],
          include_fxlang: true,
        }),
      ).toBe("resource:Rain:condition");
    });
  });
});
