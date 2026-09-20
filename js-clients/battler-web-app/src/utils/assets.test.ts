import { describe, expect, it } from "vitest";
import { itemIconUrl, monIconUrl, monRenderUrl } from "./assets";

describe("assets helper", () => {
  it("generates correct monRenderUrl", () => {
    expect(monRenderUrl("Charizard")).toBe("/assets/mons/renders/charizard.webp");
    expect(monRenderUrl("Mega Abomasnow")).toBe("/assets/mons/renders/megaabomasnow.webp");
    expect(monRenderUrl("Oinkologne-Female")).toBe("/assets/mons/renders/oinkolognefemale.webp");
  });

  it("generates correct monIconUrl", () => {
    expect(monIconUrl("Pikachu")).toBe("/assets/mons/icons/pikachu.png");
    expect(monIconUrl("Charizard-Mega-X")).toBe("/assets/mons/icons/charizardmegax.png");
  });

  it("generates correct itemIconUrl", () => {
    expect(itemIconUrl("Leftovers")).toBe("/assets/items/leftovers.png");
    expect(itemIconUrl("Choice Scarf")).toBe("/assets/items/choicescarf.png");
    expect(itemIconUrl("King's Rock")).toBe("/assets/items/kingsrock.png");
    expect(itemIconUrl("Poké Ball")).toBe("/assets/items/pokeball.png");
    expect(itemIconUrl("pokeball")).toBe("/assets/items/pokeball.png");
  });

  it("handles accented names without dropping decomposed characters", () => {
    expect(monRenderUrl("Flabébé")).toBe("/assets/mons/renders/flabebe.webp");
    expect(monIconUrl("Flabébé")).toBe("/assets/mons/icons/flabebe.png");
  });
});
