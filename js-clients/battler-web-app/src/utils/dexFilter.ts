import { toId } from "./dataTooltipFormatting";
import type {
  AbilitySummary,
  ItemSummary,
  MoveSummary,
  SpeciesSummary,
} from "../hooks/useDexCatalog";

export interface DexFilterOptions {
  query?: string;
  type?: string | null;
  selectedTypes?: string[];
  category?: string | null;
}

export function filterSpecies(
  species: SpeciesSummary[],
  options: DexFilterOptions,
): SpeciesSummary[] {
  const queryId = options.query ? toId(options.query) : "";
  const types = options.selectedTypes?.length
    ? options.selectedTypes.map((t) => t.trim().toLowerCase())
    : options.type && options.type !== "all"
      ? [options.type.trim().toLowerCase()]
      : [];

  return species.filter((s) => {
    if (queryId) {
      const nameId = toId(s.name);
      if (!nameId.includes(queryId) && !s.id.includes(queryId)) {
        return false;
      }
    }
    if (types.length === 1) {
      const target = types[0];
      const pType = s.primary_type.toLowerCase();
      const sType = s.secondary_type?.toLowerCase();
      if (pType !== target && sType !== target) {
        return false;
      }
    } else if (types.length >= 2) {
      const [t1, t2] = types;
      const pType = s.primary_type.toLowerCase();
      const sType = s.secondary_type?.toLowerCase();
      const matches = (pType === t1 && sType === t2) || (pType === t2 && sType === t1);
      if (!matches) {
        return false;
      }
    }
    return true;
  });
}

export function filterMoves(
  moves: MoveSummary[],
  options: DexFilterOptions,
): MoveSummary[] {
  const queryId = options.query ? toId(options.query) : "";
  const types = options.selectedTypes?.length
    ? options.selectedTypes.map((t) => t.trim().toLowerCase())
    : options.type && options.type !== "all"
      ? [options.type.trim().toLowerCase()]
      : [];
  const selectedCategory = options.category?.trim().toLowerCase();

  return moves.filter((m) => {
    if (queryId) {
      const nameId = toId(m.name);
      if (!nameId.includes(queryId) && !m.id.includes(queryId)) {
        return false;
      }
    }
    if (types.length > 0) {
      if (!types.includes(m.primary_type.toLowerCase())) {
        return false;
      }
    }
    if (selectedCategory && selectedCategory !== "all") {
      if (m.category.toLowerCase() !== selectedCategory) {
        return false;
      }
    }
    return true;
  });
}

export function filterAbilities(
  abilities: AbilitySummary[],
  query?: string,
): AbilitySummary[] {
  const queryId = query ? toId(query) : "";
  if (!queryId) return abilities;
  return abilities.filter((a) => {
    return toId(a.name).includes(queryId) || a.id.includes(queryId);
  });
}

export function compareResourceNames(a: string, b: string): number {
  const aSym = !/^[a-zA-Z0-9]/.test(a);
  const bSym = !/^[a-zA-Z0-9]/.test(b);
  if (aSym && !bSym) return 1;
  if (!aSym && bSym) return -1;
  return a.localeCompare(b, undefined, { sensitivity: "base" });
}

export function filterItems(
  items: ItemSummary[],
  query?: string,
): ItemSummary[] {
  const queryId = query ? toId(query) : "";
  const filtered = items.filter((i) => {
    // Exclude unusable dynamax crystals (★And15, etc.) unless specifically queried
    if (!queryId && (i.id.startsWith("dynamaxcrystal") || i.name.startsWith("★"))) {
      return false;
    }
    if (queryId) {
      return toId(i.name).includes(queryId) || i.id.includes(queryId);
    }
    return true;
  });

  return filtered.sort((a, b) => compareResourceNames(a.name, b.name));
}

export function paginateList<T>(
  items: T[],
  page: number,
  pageSize: number,
): {
  pageItems: T[];
  totalPages: number;
  totalItems: number;
} {
  const totalItems = items.length;
  const totalPages = Math.max(1, Math.ceil(totalItems / pageSize));
  const validPage = Math.min(Math.max(1, page), totalPages);
  const start = (validPage - 1) * pageSize;
  const pageItems = items.slice(start, start + pageSize);

  return {
    pageItems,
    totalPages,
    totalItems,
  };
}
