import { beforeEach, describe, expect, it, vi } from "vitest";
import * as wampModule from "../core/wamp";
import {
  fetchDexCatalog,
  getCachedCatalog,
  resetCachedCatalogForTesting,
  setCachedCatalogForTesting,
} from "./useDexCatalog";

describe("useDexCatalog", () => {
  const mockCatalog = {
    species: [
      { id: "bulbasaur", name: "Bulbasaur", primary_type: "Grass", secondary_type: "Poison" },
    ],
    moves: [
      { id: "tackle", name: "Tackle", primary_type: "Normal", category: "Physical" },
    ],
    abilities: [
      { id: "overgrow", name: "Overgrow", description: "Powers up Grass-type moves." },
    ],
    items: [
      { id: "leftovers", name: "Leftovers", description: "Restores HP gradually." },
    ],
  };

  const mockClient = {
    getCatalog: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    resetCachedCatalogForTesting();
    (wampModule.connectionManager as any).dataServiceClient = null;
  });

  it("returns default catalog when client is not available", async () => {
    const res = await fetchDexCatalog();
    expect(res).toBeDefined();
    expect(res.species.length).toBeGreaterThan(0);
    expect(res.moves.length).toBeGreaterThan(0);
  });

  it("fetches catalog from server when client is available", async () => {
    (wampModule.connectionManager as any).dataServiceClient = mockClient;
    mockClient.getCatalog.mockResolvedValueOnce(mockCatalog);

    const res = await fetchDexCatalog();
    expect(res.species).toEqual(mockCatalog.species);
    expect(res.moves).toEqual(mockCatalog.moves);
    expect(getCachedCatalog().species).toEqual(mockCatalog.species);
  });

  it("falls back to default catalog when server fetch fails", async () => {
    (wampModule.connectionManager as any).dataServiceClient = mockClient;
    mockClient.getCatalog.mockRejectedValueOnce(new Error("RPC failed"));

    const res = await fetchDexCatalog();
    expect(res).toBeDefined();
    expect(res.species.length).toBeGreaterThan(0);
  });

  it("supports setCachedCatalogForTesting and resetCachedCatalogForTesting", () => {
    setCachedCatalogForTesting(mockCatalog);
    expect(getCachedCatalog().species).toEqual(mockCatalog.species);

    resetCachedCatalogForTesting();
    expect(getCachedCatalog().species.length).toBeGreaterThan(100);
  });
});
