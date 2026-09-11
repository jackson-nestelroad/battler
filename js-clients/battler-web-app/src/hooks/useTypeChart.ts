import { useEffect, useState } from "react";
import type { TypeChartData } from "battler-types";
import { connectionManager } from "../core/wamp";

export const ALL_POKEMON_TYPES = [
  "Normal",
  "Fire",
  "Water",
  "Electric",
  "Grass",
  "Ice",
  "Fighting",
  "Poison",
  "Ground",
  "Flying",
  "Psychic",
  "Bug",
  "Rock",
  "Ghost",
  "Dragon",
  "Dark",
  "Steel",
  "Fairy",
] as const;

export type PokemonType = (typeof ALL_POKEMON_TYPES)[number];

let cachedTypeChart: TypeChartData | null = null;
let pendingPromise: Promise<TypeChartData> | null = null;

export function getCachedTypeChart(): TypeChartData | null {
  return cachedTypeChart;
}

export function setCachedTypeChartForTesting(data: TypeChartData | null): void {
  cachedTypeChart = data;
  pendingPromise = null;
}

export async function fetchTypeChart(): Promise<TypeChartData> {
  if (cachedTypeChart) return cachedTypeChart;
  if (pendingPromise) return pendingPromise;

  const dataClient = connectionManager.dataServiceClient;
  if (!dataClient) {
    throw new Error("Data client is not connected");
  }

  const promise = dataClient
    .getTypeChart()
    .then((data: TypeChartData) => {
      cachedTypeChart = data;
      pendingPromise = null;
      return data;
    })
    .catch((err: unknown) => {
      pendingPromise = null;
      throw err;
    });

  pendingPromise = promise;
  return promise;
}

export function getEffectiveness(
  typeChart: TypeChartData | null,
  attacker: string,
  defender: string,
): number {
  if (!typeChart?.types) return 1;
  const atkMap = typeChart.types[attacker];
  if (!atkMap) return 1;
  const val = atkMap[defender];
  return typeof val === "number" ? val : 1;
}

export function getDefensiveMultipliers(
  typeChart: TypeChartData | null,
  defenders: string[],
): Record<string, number> {
  const cleanDefenders = defenders.filter(Boolean);
  const result: Record<string, number> = {};

  for (const atk of ALL_POKEMON_TYPES) {
    if (cleanDefenders.length === 0) {
      result[atk] = 1;
      continue;
    }
    let mult = 1;
    for (const def of cleanDefenders) {
      mult *= getEffectiveness(typeChart, atk, def);
    }
    result[atk] = mult;
  }

  return result;
}

export function getOffensiveMultipliers(
  typeChart: TypeChartData | null,
  attacker: string,
): Record<string, number> {
  const result: Record<string, number> = {};
  for (const def of ALL_POKEMON_TYPES) {
    result[def] = getEffectiveness(typeChart, attacker, def);
  }
  return result;
}

export function formatMultiplier(multiplier: number): string {
  if (multiplier === 0) return "0×";
  if (multiplier === 0.25) return "¼×";
  if (multiplier === 0.5) return "½×";
  if (multiplier === 2) return "2×";
  if (multiplier === 4) return "4×";
  return "1×";
}

export interface UseTypeChartResult {
  typeChart: TypeChartData | null;
  loading: boolean;
  error: string | null;
}

export function useTypeChart(): UseTypeChartResult {
  const [typeChart, setTypeChart] = useState<TypeChartData | null>(cachedTypeChart);
  const [loading, setLoading] = useState<boolean>(!cachedTypeChart);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (cachedTypeChart) {
      setTypeChart(cachedTypeChart);
      setLoading(false);
      return;
    }

    let isMounted = true;
    setLoading(true);
    fetchTypeChart()
      .then((data) => {
        if (isMounted) {
          setTypeChart(data);
          setError(null);
          setLoading(false);
        }
      })
      .catch((err) => {
        if (isMounted) {
          setError(err?.message || "Failed to load type chart");
          setLoading(false);
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  return { typeChart, loading, error };
}
