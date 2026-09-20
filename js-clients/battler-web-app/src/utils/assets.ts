import { toId } from "./dataTooltipFormatting";

const getBaseUrl = (): string => import.meta.env?.BASE_URL ?? "/";

export function monRenderUrl(species: string): string {
  return `${getBaseUrl()}assets/mons/renders/${toId(species)}.webp`;
}

export function monIconUrl(species: string): string {
  return `${getBaseUrl()}assets/mons/icons/${toId(species)}.png`;
}

export function itemIconUrl(item: string): string {
  return `${getBaseUrl()}assets/items/${toId(item)}.png`;
}
