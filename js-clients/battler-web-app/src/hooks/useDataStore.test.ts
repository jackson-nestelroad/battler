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
  fetchItem,
  fetchMove,
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
});
