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

const SUBSCRIPT_DIGITS: Record<string, string> = {
  "0": "₀",
  "1": "₁",
  "2": "₂",
  "3": "₃",
  "4": "₄",
  "5": "₅",
  "6": "₆",
  "7": "₇",
  "8": "₈",
  "9": "₉",
};

/**
 * Formats a fraction (or multiplier < 1) using unicode vulgar fractions or
 * superscript/subscript small number variants.
 *
 * Supported exact powers of two:
 * - 1/2   => ½
 * - 1/4   => ¼
 * - 1/8   => ⅛
 * - 1/16  => ¹⁄₁₆
 * - 1/32  => ¹⁄₃₂
 * - 1/64  => ¹⁄₆₄
 * - 1/128 => ¹⁄₁₂₈
 *
 * Any other fraction uses superscript "¹" + fraction slash "⁄" + subscript denominator digits.
 */
export function formatFraction(mult: number): string {
  if (mult === 0.5) return "½";
  if (mult === 0.25) return "¼";
  if (mult === 0.125) return "⅛";
  if (mult === 0.0625) return "¹⁄₁₆";
  if (mult === 0.03125) return "¹⁄₃₂";
  if (mult === 0.015625) return "¹⁄₆₄";
  if (mult === 0.0078125) return "¹⁄₁₂₈";

  if (mult > 0 && mult < 1) {
    const denom = Math.round(1 / mult);
    const sub = denom
      .toString()
      .split("")
      .map((d) => SUBSCRIPT_DIGITS[d] ?? d)
      .join("");
    return `¹⁄${sub}`;
  }

  return mult.toString();
}

/**
 * Formats a multiplier value into a numeric string or fraction representation,
 * optionally returning null for neutral (1×) multipliers.
 */
export function formatMultiplierValue(
  multiplier: number,
  showNeutral = false,
): string | null {
  if (multiplier === 0) return "0";
  if (multiplier === 1) return showNeutral ? "1" : null;
  if (multiplier > 0 && multiplier < 1) return formatFraction(multiplier);
  if (multiplier > 1) return multiplier.toString();
  return null;
}

export function formatMultiplier(multiplier: number): string {
  const val = formatMultiplierValue(multiplier, true);
  return val !== null ? `${val}×` : "1×";
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
